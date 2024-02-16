use alloc::string::String;
use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;
use x86_64::instructions::interrupts;

lazy_static! {
    pub static ref DEFAULT_COLOR: ColorCode = ColorCode::new(Color::White, Color::Black);
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: *DEFAULT_COLOR,
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;

const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            0x0e => {
                self.write_byte(b'b');
            }
            0x08 => {
                self.remove_last_byte();
            }
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;
                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                byte => self.write_byte(byte),
            }
        }
    }

    fn remove_last_byte(&mut self) {
        if self.column_position > 0 {
            self.column_position -= 1;
            self.write_byte(b' ');
            self.column_position -= 1;
        } else {
            self.move_down();
            self.set_col_pos();
        }
    }

    fn set_col_pos(&mut self) {
        self.column_position = BUFFER_WIDTH - 1;
        while (self.column_position as i8) >= 0 {
            let current = self.buffer.chars[BUFFER_HEIGHT - 1][self.column_position]
                .read()
                .ascii_character as u8;
            match current {
                0x21..=0x7e => {
                    break;
                }
                _ => {
                    self.column_position -= 1;
                }
            }
        }
        self.column_position += 1;
    }

    fn move_down(&mut self) {
        let mut row: i8 = (BUFFER_HEIGHT - 2) as i8;
        while row >= 0 {
            for col in 0..BUFFER_WIDTH {
                let ch: ScreenChar = self.buffer.chars[row as usize][col].read();
                self.buffer.chars[(row + 1) as usize][col].write(ch);
            }
            row -= 1;
        }
    }

    fn new_line(&mut self) {
        let mut command = String::from("");
        for col in 0..=self.column_position {
            command.push(
                self.buffer.chars[BUFFER_HEIGHT - 1][col]
                    .read()
                    .ascii_character as char,
            );
        }
        self.move_up();
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;

        self.color_code = ColorCode::new(Color::Green, Color::Black);
        self.write_string(&command);
        self.color_code = ColorCode::new(Color::White, Color::Black);
        self.move_up();
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn move_up(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

pub fn change_color(cc: ColorCode) {
    WRITER.lock().color_code = cc;
}

pub fn reset_color() {
    WRITER.lock().color_code = *DEFAULT_COLOR;
}

#[macro_export]
#[allow(unreachable_code)]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

#[test_case]
fn test_println_output() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    let s = "Some test string that fits on a single line";
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{}", s).expect("writeln failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.buffer.chars[BUFFER_HEIGHT - 2][i].read();
            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    });
}
