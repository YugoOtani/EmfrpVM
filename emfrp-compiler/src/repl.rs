use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::iter;
use std::thread;
use std::time::Duration;

use crate::compile::compile_common::*;
use crate::insn::to_byte_code;
use crate::insn::ObjHeader;
use crate::parser::*;
use crate::*;
use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;

use serial2::*;
const CONSOLE: &str = " > ";
const CONSOLE2: &str = "...";
pub const BUF_SIZE: usize = 1024;
const DEBUG_COMPILER: bool = false;
const DEBUG_FILE: &str = "machine.txt";
enum Out {
    UART(SerialPort),
    FILE(String), //for debug
}
const RES_TABLE: [&str; 5] = ["OK", "RUNTIME ERROR", "PANIC", "TODO", "OUT OF MEMORY"];
impl Out {
    pub fn truncate(&mut self) {
        match self {
            Out::UART(port) => port.discard_buffers().unwrap(),
            Out::FILE(f) => {
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&f)
                    .unwrap();
            }
        };
    }
    fn write(&mut self, v: &Vec<u8>) -> Result<()> {
        match self {
            Out::UART(port) => port.write_all(v).context("could not write to the port"),
            Out::FILE(f) => match OpenOptions::new().write(true).truncate(true).open(&f) {
                Err(_) => bail!("could not open file"),
                Ok(mut fd) => fd.write_all(v).context("could not write to the file"),
            },
        }
    }
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            Out::UART(port) => port.read(buf).context("could not read port"),
            Out::FILE(f) => match OpenOptions::new().read(true).open(&f) {
                Err(_) => bail!("could not open file"),
                Ok(mut fd) => fd.read(buf).context("could not read file"),
            },
        }
    }
}
fn read_u32(v: &[u8]) -> Result<u32> {
    if v.len() < 4 {
        bail!("UART error");
    }
    let v: [u8; 4] = [v[0], v[1], v[2], v[3]];
    Ok(u32::from_le_bytes(v))
}
fn read_i32(v: &[u8]) -> Result<i32> {
    if v.len() < 4 {
        bail!("UART error");
    }
    let v: [u8; 4] = [v[0], v[1], v[2], v[3]];
    Ok(i32::from_le_bytes(v))
}
fn u8_to_value(t: &Type, v: &[u8], res: &mut String) -> Result<usize> {
    match t {
        Type::Int => {
            res.push_str(&read_i32(v)?.to_string());
            Ok(4)
        }
        Type::Bool => {
            let i = read_i32(v)?;
            res.push_str(if i == 0 { "false" } else { "true" });
            Ok(4)
        }
        Type::User(_, vars) => {
            let header = ObjHeader(read_u32(v)?);
            let (tag, _, _) = header.decode();
            if tag > vars.len() as u32 || tag == 0 {
                bail!("UART error");
            }
            let &(ref vname, ref vargs) = &vars[tag as usize - 1];
            res.push_str(&vname.0);
            if vargs.len() == 0 {
                return Ok(4);
            }
            let mut i = 4;
            res.push('(');
            for (x, t) in vargs.iter().enumerate() {
                if t.is_obj_type() {
                    let j = u8_to_value(t, &v[i..], res)?;
                    i += j;
                } else {
                    let j = u8_to_value(t, &v[i..], res)?;
                    assert_eq!(j, 4);
                    i += 4;
                }
                if x != vargs.len() - 1 {
                    res.push(',');
                }
            }
            res.push(')');
            Ok(i)
        }
        Type::Tuple(types) => {
            if types.len() == 0 {
                return Ok(4);
            }
            let mut i = 4;
            res.push('(');
            for (x, t) in types.iter().enumerate() {
                if t.is_obj_type() {
                    let j = u8_to_value(t, &v[i..], res)?;
                    i += j;
                } else {
                    let j = u8_to_value(t, &v[i..], res)?;
                    assert_eq!(j, 4);
                    i += 4;
                }
                if x != types.len() - 1 {
                    res.push(',');
                }
            }
            res.push(')');
            Ok(i)
        }
    }
}
pub struct Repl {
    pub cmp: Compiler,
    parser: ParserWrapper,
    port: Out,
}
impl Repl {
    pub fn run(mut self) {
        for _ in 0.. {
            stdout().flush().unwrap();
            print!("{CONSOLE}");
            stdout().flush().unwrap();
            let mut input = String::new();
            stdin().read_line(&mut input).unwrap();
            let cmp_clone = self.cmp.clone();
            match self.interpret(&input) {
                Ok(res) => println!("{res}"),
                Err(msg) => {
                    self.cmp = cmp_clone;
                    println!("{:?}", msg);
                }
            }
        }
    }
    pub fn new() -> Result<Self, String> {
        if DEBUG_COMPILER {
            return Ok(Self::new_with_file());
        }
        let mut port = SerialPort::open(UART_FILE, BAUD_RATE).map_err(|e| format!("{:?}", e))?;
        let mut settings = port.get_configuration().map_err(|e| format!("{:?}", e))?;

        settings.set_stop_bits(StopBits::One);
        settings.set_flow_control(FlowControl::None);
        settings.set_char_size(CharSize::Bits8);
        port.set_configuration(&settings)
            .map_err(|e| format!("{:?}", e))?;
        port.set_read_timeout(Duration::from_secs(10))
            .map_err(|e| format!("{:?}", e))?;
        Ok(Self {
            cmp: Compiler::new(),
            parser: ParserWrapper::new(),
            port: Out::UART(port),
        })
    }
    pub fn new_with_file() -> Self {
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(DEBUG_FILE)
            .unwrap();
        Self {
            cmp: Compiler::new(),
            parser: ParserWrapper::new(),
            port: Out::FILE(DEBUG_FILE.to_string()),
        }
    }
    //output gpio5
    pub fn add_input_node(&mut self, name: &'static str, typ: Type) {
        self.cmp.add_input_node(name, typ);
    }
    pub fn add_output_node(&mut self, name: &'static str, typ: Type) {
        self.cmp.add_output_node(name, typ);
    }
}
fn load_file(fname: &str) -> Result<String> {
    let mut f = File::open(fname).context("file not found")?;

    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("something went wrong reading the file");
    Ok(contents)
}
impl Repl {
    pub fn interpret(&mut self, input: &str) -> Result<String> {
        let prog = match input.trim() {
            "{" => loop {
                let mut input2 = String::new();
                stdout().flush().unwrap();
                print!("{CONSOLE2}");
                stdout().flush().unwrap();
                stdin().read_line(&mut input2).unwrap();
                if let "}" = input2.trim() {
                    break self.parser.get_result();
                } else {
                    self.parser
                        .parse_one_of_lines(&input2)
                        .map_err(|s| anyhow!("{s}"))?;
                }
            },
            input => {
                let mut iter = input.split_whitespace();
                match iter.next() {
                    None => return Ok("".to_string()),
                    Some("load") => match iter.next() {
                        Some(f) => {
                            let f_input = load_file(f)?;

                            self.parser
                                .parse_program(&f_input)
                                .map_err(|s| anyhow!("{s}"))?
                        }
                        None => bail!("usage : load <filename>"),
                    },
                    _ => self.parser.parse_line(input).map_err(|s| anyhow!("{s}"))?,
                }
            }
        };
        let res = self.cmp.compile(prog).map_err(|msg| anyhow!("{:?}", msg))?;
        match res {
            Some(code) => {
                if DEBUG {
                    println!("{:?}", code);
                }

                let bc = to_byte_code(&code);
                if DEBUG {
                    println!("");
                    println!("size : {}bytes", bc.len());
                    println!("{:?}", bc);
                }
                self.port.truncate();
                self.port.write(&bc).context("fail to write file")?;
                if DEBUG_COMPILER {
                    return Ok("ok".to_string());
                }

                match code {
                    CompiledCode::Eval(t, _) => {
                        let mut read_len = 0;
                        let num_timeout = 10;
                        let mut buf: Vec<u8> = iter::repeat(0).take(BUF_SIZE).collect();
                        for _ in 0..num_timeout {
                            read_len += self.port.read(&mut buf[read_len..])?;
                            if read_len > 0 {
                                let ret_val_size = buf[0] as usize;
                                for _ in 0..num_timeout {
                                    // datasize(u8) data status(u8)
                                    if read_len >= ret_val_size + 2 {
                                        if DEBUG {
                                            println!("{:?}", &buf[0..read_len]);
                                        }
                                        let status = buf[ret_val_size + 1];
                                        if status >= RES_TABLE.len() as u8 {
                                            bail!("UART error")
                                        } else {
                                            if status != 0 {
                                                bail!("{}", RES_TABLE[status as usize])
                                            } else {
                                                let mut s = String::new();
                                                u8_to_value(&t, &buf[1..], &mut s)?;
                                                return Ok(format!("[OK] {s}"));
                                            }
                                        }
                                    }
                                    read_len += self.port.read(&mut buf[read_len..])?;
                                    thread::sleep(Duration::from_millis(10));
                                }
                                bail!("UART timeout. Plese try again");
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                        bail!("UART timeout. Plese try again");
                    }
                    CompiledCode::Def(_) => {
                        let num_timeout = 10;
                        let mut read_len = 0;
                        let mut buf: Vec<u8> = iter::repeat(0).take(BUF_SIZE).collect();
                        for _ in 0..num_timeout {
                            read_len += self.port.read(&mut buf[read_len..])?;
                            if read_len > 0 {
                                let status = buf[0];
                                if status >= RES_TABLE.len() as u8 {
                                    bail!("UART error")
                                } else {
                                    if status != 0 {
                                        bail!("{}", RES_TABLE[status as usize])
                                    } else {
                                        return Ok(RES_TABLE[status as usize].to_string());
                                    }
                                }
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                        bail!("UART timeout. Plese try again");
                    }
                }
            }
            None => return Ok("defined successfully".to_string()),
        }
    }
}
