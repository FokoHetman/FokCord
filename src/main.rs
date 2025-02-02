mod foklang;
use {
  foklang::foklang::Foklang, json::{self, object, JsonValue}, libc, std::{env, fs, io::{self,IsTerminal,Read, Write}, path::Path, process::Command, str, sync::{Arc,Mutex}}, unicode_segmentation::UnicodeSegmentation
};

static termios: Mutex<libc::termios> = Mutex::new(libc::termios { c_iflag: 0, c_oflag: 0, c_cflag: 0, c_lflag: 0, c_line: 1, c_cc: [0 as u8; 32], c_ispeed: 1, c_ospeed: 1 });

fn setup_termios() {
  termios.lock().unwrap().c_cflag &= !libc::CSIZE;
  termios.lock().unwrap().c_cflag |= libc::CS8;
  termios.lock().unwrap().c_cc[libc::VMIN] = 1;
}

extern "C" fn disable_raw_mode() {
  unsafe {
    libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &(*termios.lock().unwrap()));
  }
}
fn enable_raw_mode() {
  unsafe {
    libc::tcgetattr(libc::STDIN_FILENO, &mut *termios.lock().unwrap());
    libc::atexit(disable_raw_mode);
    let mut raw = *termios.lock().unwrap();
    raw.c_lflag &= !(libc::ECHO | libc::ICANON);
    libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &raw);
  }
}

#[derive(Debug,Clone,PartialEq)]
pub struct KeyEvent {
  pub code: KeyCode,
  pub modifiers: Vec<Modifier>,
}
#[derive(Debug,PartialEq,Clone)]
pub enum Modifier {
  Control,
  Shift,
  //why even do it at this point
}
#[derive(Debug,PartialEq,Clone)]
pub enum Direction {
  Up,
  Down,
  Right,
  Left
}

#[derive(Debug,PartialEq,Clone)]
pub enum KeyCode {
  Escape,
  Colon,
  Enter,
  Tab,
  Backspace,
  Delete,
  Arrow(Direction),
  Char(char),
}

const ESCAPE: char = 27 as char;
const BACKSPACE: char = '\u{7f}';
const TAB: char = '\t';
const ENTER: char = '\n';

fn getch() -> char {
  io::stdin().bytes().next().unwrap().unwrap() as char
}
fn get_arrow() -> KeyCode {
  match getch() {'A' => KeyCode::Arrow(Direction::Up), 'B' => KeyCode::Arrow(Direction::Down), 'C' => KeyCode::Arrow(Direction::Right), 'D' => KeyCode::Arrow(Direction::Left),
                                                           _ => KeyCode::Escape }
}




#[repr(C)]              /// (github.com) softprops/termsize!!
#[derive(Debug)]
pub struct UnixSize {
    pub rows: libc::c_ushort,
    pub cols: libc::c_ushort,
    x: libc::c_ushort,
    y: libc::c_ushort,
}

pub struct TerminalSize {pub rows: u16, pub cols: u16}

fn get_terminal_size() -> Option<TerminalSize> {
  if !std::io::stdout().is_terminal() {
    return None;
  }
  let mut us = UnixSize { // we don't support windows here
    rows: 0,
    cols: 0,
    x: 0,
    y: 0,
  };
  let r = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut us) };
  if r == 0 {
    Some(TerminalSize{
      rows: us.rows,
      cols: us.cols,
    })
  } else {
    None
  }
}

#[derive(Debug,PartialEq,Clone)]
enum State {
  Control,
  Command,
  Message,
}

#[derive(Debug,PartialEq,Clone)]
struct Fokcord {
  state: State,
  guilds: Vec<Guild>,
  current_guild: usize,
  io: String,
  io_cursor: usize,
  exit: bool,
  token: String,
  foklang: Foklang,
  icon_cache: Vec<(String, String)>,
}

impl Fokcord {
  fn reload_cache(&mut self) {
    for i in &self.guilds {
      let path = env::home_dir().unwrap().to_str().unwrap().to_string() + &format!("/.cache/fokcord/guilds/{}.png", i.icon);
      println!("{}", format!("curl https://cdn.discordapp.com/icons/{}/{}.png", i.id, i.icon));
      let img_data = Command::new("sh").arg("-c").arg(format!("curl https://cdn.discordapp.com/icons/{}/{}.png", i.id, i.icon)).output().unwrap().stdout;
      let _ = fs::write(path.clone(), img_data).unwrap();
      /*let cmd = Command::new("sh").arg("-c").arg(format!("kitten icat {}", path)).output().unwrap();
      println!("technically an icon: {}", str::from_utf8(&cmd.stdout).unwrap());*/
    }
    self.gen_icon_cache();
  }
  fn gen_icon_cache(&mut self) {
    for i in &self.guilds {
      let path = env::home_dir().unwrap().to_str().unwrap().to_string() + &format!("/.cache/fokcord/guilds/{}.png", i.icon);
      let cmd = Command::new("sh").arg("-c").arg(format!("kitten icat {}", path)).output().unwrap();
      self.icon_cache.push((i.icon.clone(), str::from_utf8(&cmd.stdout).unwrap().to_string()));
    }
  }
  fn get_message(&mut self, id: i64) -> JsonValue {
    let list = &self.get_guild().get_channel().messages;
    let id = id.to_string();
    for i in 0..list.len() {
      if list[i]["id"].to_string() == id {
        return list[i].clone()
      }
    }
    
    /*let message = json::parse(&mk_get_request(&format!("channels/{}/messages/{}", self.get_guild().get_channel().id, id), 
            vec![],
            vec![("Content-Type", "application/json"), ("Authorization", &self.token), ("Content-Disposition", "form-data; name=\\\"content\\\"")])).unwrap(); 

    self.get_guild().get_channel().cached_messages.push(message.clone());*/
    object! {content: "[can't load]", author: object! {username: "[can't load]"}}
  }
  fn display(&mut self) {
    self.clear();
    let size = get_terminal_size().unwrap();
    let mut result = String::new();


    let title = "Fokcord - please send help";
    let location = self.get_guild().name.clone() + "//" + &self.get_guild().get_channel().name;
    
    result += &(title.to_owned() + &vec![" "; size.cols as usize - title.len()].into_iter().collect::<String>() + "\n");
    
    let serverlist_offset: usize = 5;
    let channellist_offset: usize = 45;
    

    result += &(vec![" "; serverlist_offset].into_iter().collect::<String>()  + &location + &vec![" "; size.cols as usize - serverlist_offset - location.len()].into_iter().collect::<String>() + "\n");

    for i in 4..size.rows as usize { // messages, someday
      let channel_text;
      if self.get_guild().channels.len()>i-4 {
        
        channel_text = if self.get_guild().channels[i-4].name.len() > channellist_offset {
          self.get_guild().channels[i-4].name[..channellist_offset].to_string()
        } else {
          self.get_guild().channels[i-4].name.clone()
        }
      } else {
        channel_text = vec![" "; channellist_offset].into_iter().collect::<String>();
      }
      result += &(vec![" "; serverlist_offset].into_iter().collect::<String>() + &channel_text + &vec![" "; size.cols as usize - serverlist_offset - channellist_offset].into_iter().collect::<String>() + "\n");
    }

    result += &(vec![" "; serverlist_offset+channellist_offset].into_iter().collect::<String>() + &self.get_guild().get_channel().message_box + 
      &vec![" "; size.cols as usize - serverlist_offset - channellist_offset - self.get_guild().get_channel().message_box.len()].into_iter().collect::<String>() + "\n"); // message box 
  


    let state = match self.state {
      State::Control => "control",
      State::Message => "message",
      State::Command => "command",
    };
    result += &(String::from("\x1b[48;2;43;48;40m") + &self.io + &vec![" "; size.cols as usize - self.io.len() - state.len()].into_iter().collect::<String>() + state + "\x1b[0m");
    

    let mut y = (size.rows-2) as usize;
    result += &format!("\x1b[{y};{}H", serverlist_offset + channellist_offset + 1);

    let messages = self.get_guild().get_channel().messages.clone();

    let mut can_fit = ((size.rows-4)/2) as usize;
    /*if self.get_guild().get_channel().messages.len()==0 {
      let id = self.get_guild().current_channel;
      self.fetch_messages(self.current_guild, id);
    }*/
    if can_fit > self.get_guild().get_channel().messages.len() {
      can_fit = self.get_guild().get_channel().messages.len();
    }

    if messages.has_key("code") {
      result += &messages["message"].to_string();
    } else {

    //println!("{}", self.get_guild().get_channel().messages.len());
    let scroll = 0;
    for i in scroll..scroll+can_fit {

      let author = messages[i]["author"]["username"].to_string();

      //result += &format!("\x1b[{y};{}H", serverlist_offset + channellist_offset + 1);
      let mut content = messages[i]["content"].to_string();
      let max_line_len = size.cols as usize - serverlist_offset - channellist_offset - 1;

      if content.len() == 0 {
        /*if messages[i].has_key("embeds") {
          content = "[embed/s]".to_string();
        } else */if messages[i].has_key("files") {
          content = "[file/s]".to_string();
        } else {
          content = "\x1b[38;255;255;0m[empty]\x1b[0m".to_string();
        }
      }
      let replied;
      if messages[i].has_key("message_reference") {
        replied = (true, self.get_message(messages[i]["message_reference"]["message_id"].to_string().parse::<i64>().unwrap()));
      } else {
        replied = (false, JsonValue::Null);
      }

      let mut rcontents = content.split("\n").collect::<Vec<&str>>();
      let mut contents: Vec<&str> = vec![];
      for i in 0..rcontents.len() { while rcontents[i].len()>max_line_len {contents.push(rcontents[i].split_at(max_line_len).0); rcontents[i] = rcontents[i].split_at(max_line_len).1; }; contents.push(rcontents[i])}
      
      
      //let mut content_len = 0;
      for i in contents.clone().into_iter().rev() {
        result += i;
        if y < 1 {
          break
        }
        y-=1;
        result += &format!("\x1b[{y};{}H", serverlist_offset + channellist_offset + 1);
        //content_len+=1;
        
      }
      if y > 0 {
        
        result += &("\x1b[38;2;255;0;0m".to_string() + &author + "\x1b[0m");
        y-=1;
        result += &format!("\x1b[{y};{}H", serverlist_offset + channellist_offset + 1);
      }
      if y>0 {
        if replied.0 {
          let mut content = replied.1["content"].to_string();
          if content.len() == 0 {
            /*if messages[i].has_key("embeds") {
              content = "[embed/s]".to_string();
            } else */if messages[i].has_key("files") {
              content = "[file/s]".to_string();
            } else {
              content = "\x1b[38;2;255;255;0m[empty]\x1b[0m".to_string();
            }
          }
          let mut rcontents = content.split("\n").collect::<Vec<&str>>();
          let mut contents: Vec<&str> = vec![];
          for i in 0..rcontents.len() { while rcontents[i].len()>max_line_len {contents.push(rcontents[i].split_at(max_line_len).0); rcontents[i] = rcontents[i].split_at(max_line_len).1; }; contents.push(rcontents[i])}
          let mut kms = String::new();
          if contents[0].chars().count()>25 { kms = contents[0].to_string().chars().collect::<Vec<char>>()[0..25].into_iter().collect::<String>(); contents[0] = &kms;}
          result += &format!("  >\x1b[38;2;255;0;0m{}\x1b[0m: {}", replied.1["author"]["username"], contents[0]);
          drop(kms);
          y-=1;
          result += &format!("\x1b[{y};{}H", serverlist_offset + channellist_offset + 1);
        }
      }
    }
    }


    match self.state {
      State::Control => {
        result += &format!("\x1b[{line};{column}H", line = size.rows, column = size.cols);
      },
      State::Command => {
        result += &format!("\x1b[{line};{column}H", line = size.rows, column=self.io_cursor+1)
      },
      State::Message => {
        result += &format!("\x1b[{line};{column}H", line = size.rows - 1, column = self.get_guild().get_channel().message_cursor+1+channellist_offset+serverlist_offset)
      },
      _ => {}
    };
    print!("{}", result);
    /*let mut pos = (0,6);
    let mut icons = String::new();
    for i in &self.guilds {
      let  path = env::home_dir().unwrap().to_str().unwrap().to_string() + &format!("/.cache/fokcord/guilds/{}.png", i.icon);
      let cmd = Command::new("sh").arg("-c").arg(format!("kitty icat --place 5x5@{}x{} {}", pos.0, pos.1, path)).output().unwrap();
      icons += str::from_utf8(&cmd.stdout).unwrap();
      pos.1 += 5;
    }
    print!("{}", icons);*/
    let _ = io::stdout().flush().unwrap();
  }
  fn clear(&self) {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush().unwrap();
  }
  fn write_io(&mut self, text: &str) {
    let mut left = self.io[..self.io_cursor].to_string();
    left += text;
    left += &self.io[self.io_cursor..];
    self.io_cursor+=text.len();
    self.io = left.to_string();
  }
  fn write_message(&mut self, text: &str) {
    let cursor = self.get_guild().get_channel().message_cursor;
    
    let mut left = self.get_guild().get_channel().message_box[..cursor].to_string();
    left += text;
    left += &self.get_guild().get_channel().message_box[cursor..];
    self.get_guild().get_channel().message_cursor+=text.len();
    self.get_guild().get_channel().message_box = left;
  }
  fn get_guild(&mut self) -> &mut Guild {
    &mut self.guilds[self.current_guild]
  }
  fn send_message(&mut self) {
    let content = self.get_guild().get_channel().message_box[1..].to_string();
    mk_post_request(&("channels/".to_string() + &self.get_guild().get_channel().id.to_string() + "/messages"), 

      vec![],
      vec![("Content-Type", "application/json"), ("Authorization", &self.token), ("Content-Disposition", "form-data; name=\\\"content\\\"")],
      format!("{{\"content\":\"{content}\"}}"));
  }

  fn evaluate_io(&mut self) -> String {
    let mut ch = self.io.chars();
    ch.next() ;
    
    let foklang = Arc::new(Mutex::new(self.foklang.clone()));
    let panics = std::panic::catch_unwind(|| {
      let mut lock = foklang.lock();
      let (program,io) = lock.as_mut().unwrap().run(ch.collect::<String>(), self.clone()); // foklang.run returns display of returned value from foklang code
      //drop(foklang);
      drop(lock);
      (program,io)}
    );
    //self.foklang.env = foklang.lock().unwrap().env.clone(); // call panic
    
    if panics.is_ok() {
      let uw = panics.unwrap();
      *self = uw.0;
      //if self.config.foklang.persistence {
      self.foklang.env = foklang.lock().unwrap().env.clone(); // persistence
      //}
      uw.1
    } else {
      //panics.unwrap();
      String::from("Foklang panicked.")
    }

  }
  fn fetch_messages(&mut self, index: usize, index2: usize) {
    let messages = json::parse(&mk_get_request(&format!("channels/{}/messages", self.guilds[index].channels[index2].id), 
            vec![("limit", "50")],
            vec![("Content-Type", "application/json"), ("Authorization", &self.token), ("Content-Disposition", "form-data; name=\\\"content\\\"")]));
    self.guilds[index].channels[index2].messages = messages.unwrap();
  }
  fn update_guild(&mut self, index: usize) {
    let guild = json::parse(&mk_get_request(&("guilds/".to_string() + &self.guilds[index].id.to_string()), 
      vec![],
      vec![("Content-Type", "application/json"), ("Authorization", &self.token), ("Content-Disposition", "form-data; name=\\\"content\\\"")]));
    
    let channels = json::parse(&mk_get_request(&("guilds/".to_string() + &self.guilds[index].id.to_string() + "/channels"), 
      vec![],
      vec![("Content-Type", "application/json"), ("Authorization", &self.token), ("Content-Disposition", "form-data; name=\\\"content\\\"")]));

    

    self.guilds[index].name = guild.unwrap()["name"].to_string();
    
    let list = channels.unwrap();
    if self.guilds[index].channels.len() == list.len() {
      for i in 0..list.len() {
      
        self.guilds[index].channels[i] = Channel{name: list[i]["name"].to_string(),
          messages: JsonValue::Null,
          message_cursor: self.guilds[index].channels[i].message_cursor, message_box: self.guilds[index].channels[i].message_box.clone(), id: list[i]["id"].to_string().parse::<i64>().unwrap()};
      }
    } else {
      self.guilds[index].channels = vec![];
      for i in 0..list.len() {
        self.guilds[index].channels.push(Channel{name: list[i]["name"].to_string(),
          messages: JsonValue::Null,
          message_cursor: 0, message_box: String::new(), id: list[i]["id"].to_string().parse::<i64>().unwrap()});
      }
    }
    let index = self.get_guild().current_channel;
    self.fetch_messages(self.current_guild, index);
  }
  fn update_guilds(&mut self) {
    let guilds = json::parse(&mk_get_request("users/@me/guilds", vec![], 
        vec![("Content-Type", "application/json"), ("Authorization", &self.token), ("Content-Disposition", "form-data; name=\\\"content\\\"")]));
    let list = guilds.unwrap();
    

    if self.guilds.len() == list.len() {
      for i in 0..list.len() {
      
        self.guilds[i] = Guild {
          id: list[i]["id"].to_string().parse::<i64>().unwrap(),
          name: list[i]["name"].to_string(),
          channels: self.guilds[i].channels.clone(),
          current_channel: self.guilds[i].current_channel,
          icon: list[i]["icon"].to_string(),
        };
      }
    } else {
      self.guilds = vec![];
      for i in 0..list.len() {
        self.guilds.push(Guild {
          id: list[i]["id"].to_string().parse::<i64>().unwrap(),
          name: list[i]["name"].to_string(),
          channels: vec![],
          current_channel: 0,
          icon: list[i]["icon"].to_string()
        });
      }
    }
    self.update_guild(self.current_guild)
  }
}

#[derive(Debug,PartialEq,Clone)]
struct Guild {
  id: i64,
  name: String,
  channels: Vec<Channel>,
  current_channel: usize,
  icon: String,
}
impl Guild {
  fn get_channel(&mut self) -> &mut Channel {
    &mut self.channels[self.current_channel]
  }
}


#[derive(Debug,PartialEq,Clone)]
struct Channel {
  id: i64,
  name: String,
  messages: JsonValue,
  message_box: String,
  message_cursor: usize,
}


fn handle_key_event(fokcord: &mut Fokcord, event: KeyEvent) -> Fokcord {

  match event.code {
    KeyCode::Colon => {
      match fokcord.state {
        State::Control => {
          fokcord.state = State::Command;
          fokcord.io = String::from(":");
          fokcord.io_cursor = 1;
        },
        State::Command => {
          fokcord.write_io(":");
        },
        State::Message => {
          fokcord.write_message(":");
        },
      }
    },
    KeyCode::Backspace => {
      match fokcord.state {
        State::Control => {},
        State::Command => {
          let mut left = fokcord.io[..fokcord.io_cursor-1].to_string();
          if fokcord.io_cursor < left.len() {
            left += &fokcord.io[fokcord.io_cursor..];
          }
          fokcord.io_cursor-=1;
          if left.len()==0 {
            fokcord.state = State::Control;
          }
          fokcord.io = left;
        },
        State::Message => {
          let cursor = fokcord.get_guild().get_channel().message_cursor;
          let mut left = fokcord.get_guild().get_channel().message_box[..cursor-1].to_string();
          if left.len()>0 {
            if cursor < left.len() {
              left += &fokcord.get_guild().get_channel().message_box[cursor..];
            }
            fokcord.get_guild().get_channel().message_cursor-=1;
          
            fokcord.get_guild().get_channel().message_box = left;
          } else {
            fokcord.state = State::Control;
          }
          
        },
      }
    },
    KeyCode::Enter => {
      match fokcord.state {
        State::Message => {
          fokcord.send_message();
          fokcord.get_guild().get_channel().message_box = String::new();
          fokcord.state = State::Control;
        },
        State::Command => {
          fokcord.io = fokcord.evaluate_io();
          fokcord.state = State::Control;
        },
        State::Control => {}
      }
    },
    KeyCode::Escape => { 
      fokcord.state = State::Control;
    },
    KeyCode::Arrow(d) => {

    },
    KeyCode::Char(c) => {
      match fokcord.state {
        State::Control => {
          match c {
            'i' | 'a' => {
              fokcord.state = State::Message;
              fokcord.get_guild().get_channel().message_box = String::from("$");
              fokcord.get_guild().get_channel().message_cursor = 1;
            },
            _ => {}
          };
        },
        State::Command => { 
          fokcord.write_io(&c.to_string());
        },
        State::Message => {
          fokcord.write_message(&c.to_string())
        },
      }
    },
    _ => {}
  }
  fokcord.clone()
}









fn mk_get_request(path: &str, parameters: Vec<(&str, &str)>, headers: Vec<(&str, &str)>) -> String {
  let mut params = String::new();
  let mut heads = String::new();
  for i in parameters {
    params += &format!(" --data-urlencode \"{}={}\"", i.0, i.1);
  }
  for i in headers {
    heads += &format!(" -H \"{}: {}\"", i.0, i.1);
  }
  println!("{}", heads);
  let cmd = Command::new("sh").arg("-c").arg(format!("curl -G {} {} https://discord.com/api/{}", params, heads, path)).output().unwrap();
  str::from_utf8(&cmd.stdout).unwrap().to_string()
}
fn mk_post_request(path: &str, parameters: Vec<(&str, &str)>, headers: Vec<(&str, &str)>, body: String) -> String {
  let mut params = String::new();
  let mut heads = String::new();
  for i in parameters {
    params += &format!(" --data-urlencode \"{}={}\"", i.0, i.1);
  }
  for i in headers {
    heads += &format!(" -H \"{}: {}\"", i.0, i.1);
  }
  
  let cURL = format!("curl --request POST {} -d '{body}' https://discord.com/api/{}",/* params,*/ heads, path);
  //panic!("{}", cURL);
  let cmd = Command::new("sh").arg("-c").arg(cURL).output().unwrap();
  str::from_utf8(&cmd.stdout).unwrap().to_string()
}


fn main() {

  let config = json::parse(&fs::read_to_string(env::home_dir().unwrap().to_str().unwrap().to_string() + "/.config/fokcord/configuration.json").unwrap()).unwrap();
  
  let token;
  if config.has_key("token") {
    token = config["token"].to_string();
  } else {
    panic!("token not defined (~/.config/fokcord/configuration.json)")
  }


  setup_termios();
  enable_raw_mode();

  let mut fokcord = Fokcord {
    state: State::Control,
    guilds: vec![],
    current_guild: 0,
    io: String::new(),
    io_cursor: 0,
    exit: false,
    token,
    foklang: Foklang::new(),
    icon_cache: vec![],
  };
  fokcord.update_guilds();
  fokcord.gen_icon_cache();
  //fokcord.reload_cache();

  //print!("\x1b 7\x1b[?47h");
  // MAIN_LOOP 

  fokcord.display();
  for b in io::stdin().bytes() {
    
    //println!("{:#?}", (*fokcord.lock().unwrap()).state);
    
    let c = b.unwrap() as char;
    //println!("{}", c);
    let mut modifiers: Vec<Modifier> = vec![];
    if c.is_control() && ![ENTER, TAB, ESCAPE, BACKSPACE].contains(&c) {
      modifiers.push(Modifier::Control);
    }
    
    let event = KeyEvent{
      code: match c { BACKSPACE => KeyCode::Backspace, ':' => KeyCode::Colon, '\n' => KeyCode::Enter,
          '\t' => KeyCode::Tab,
          'Ã' => {
            match getch() {
              '³' => {
                KeyCode::Char('ó')
              },
              '\u{93}' => {
                KeyCode::Char('Ó')
              },
              _ => KeyCode::Escape
            }
          },
          'Ä' => {
            match getch() {
              '\u{99}' => {
                KeyCode::Char('ę')
              },
              '\u{98}' => {
                KeyCode::Char('Ę')
              },
              '\u{87}' => {
                KeyCode::Char('ć')
              },
              '\u{86}' => {
                KeyCode::Char('Ć')
              },

              '\u{85}' => {
                KeyCode::Char('ą')
              },
              '\u{84}' => {
                KeyCode::Char('Ą')
              },
              _ => KeyCode::Escape
            }
          },
          'Å' => {
            match getch() {
              '\u{84}' => {
                KeyCode::Char('ń')
              },
              '\u{83}' => {
                KeyCode::Char('Ń')
              },
              '\u{82}' => {
                KeyCode::Char('ł')
              },
              '\u{81}' => {
                KeyCode::Char('Ł')
              },
              '\u{9b}' => {
                KeyCode::Char('ś')
              },
              '\u{9a}' => {
                KeyCode::Char('Ś')
              },
              'º' => {
                KeyCode::Char('ź')
              },
              '¹' => {
                KeyCode::Char('Ź')
              },
              '¼' => {
                KeyCode::Char('ż')
              },
              '»' => {
                KeyCode::Char('Ż')
              }
              _ => KeyCode::Escape
            }
          },
          '\u{1b}' => {
              match getch() { 
                    '[' => match getch() {
                        'A' => KeyCode::Arrow(Direction::Up), 'B' => KeyCode::Arrow(Direction::Down), 'C' => KeyCode::Arrow(Direction::Right), 'D' => KeyCode::Arrow(Direction::Left),
                        '1' => match getch() {
                                ';' => match getch() 
                                { '5' => {modifiers.push(Modifier::Control); get_arrow()}, '2' => {modifiers.push(Modifier::Shift); get_arrow()}, _ => KeyCode::Escape}, _ => KeyCode::Escape
                            },
                        '3' => match getch() {
                              '~' => KeyCode::Delete,
                              _ => KeyCode::Escape,
                            },
                        _ => KeyCode::Escape }, 
                    _ => KeyCode::Escape}},
          _ => KeyCode::Char(c)},
      modifiers,
    };
    //fokcord.io =  format!("{:#?}", event);
    
    
    let panics = std::panic::catch_unwind(|| {
      handle_key_event(&mut fokcord.clone(), event.clone())
    });
    
    //if panics.is_ok() { /* safety layer */
      fokcord = panics.unwrap().clone();
    /*} else {
      fokcord.io = format!("FokCord panicked trying to handle: {:#?}.", event.code);
    }*/

    //handle_key_event(&mut fokcord, event);
    if fokcord.exit {
      break;
    }
    let panics = std::panic::catch_unwind(|| {
      fokcord.clone().display();
    });
    
    if panics.is_ok() { /* safety layer */
      panics.unwrap();
    } else {
      panics.unwrap();
      fokcord.clear();
      println!("\x1b[38;2;255;0;0mError: Failed to display contents - perhaps unicode issues?\x1b[0m");
      panic!("panicked due to above reason")
    }
    fokcord.display();
  }
  fokcord.clear(); // clear exit

}
