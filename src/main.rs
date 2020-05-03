// http://devernay.free.fr/hacks/chip8/C8TECH10.HTM

extern crate sdl2;
extern crate rand;
extern crate byteorder;

use sdl2::rect::Rect;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use std::time::Instant;
use std::fs::File;
use std::env;

use rand::Rng;

use byteorder::{BigEndian, ReadBytesExt};

const STACK_SIZE: usize = 256;
const MEM_SIZE: usize = 4096;

const DISPLAY_MAX_X: usize = 64;
const DISPLAY_MAX_Y: usize = 32;
const DISPLAY_PIXEL: usize = 10;

const KEYBOARD_SIZE: usize = 16;

const DELAY_TIMER_HZ: u8 = 60;
const SOUND_TIMER_HZ: u8 = 60;

const COLOR_WHITE: Color = Color::RGB(255, 255, 255);
const COLOR_BLACK: Color = Color::RGB(0, 0, 0);

struct CHIP8 {
	vx: [u16; 16], // V registers
	i: u16, pc: u16, // I & program counter
	stack: [u16; STACK_SIZE], sp: u8,
	dt: u8,
	st: u8,
	ram: [u8; MEM_SIZE],
	display: [[u8; DISPLAY_MAX_Y]; DISPLAY_MAX_X],
	keyboard: [bool; KEYBOARD_SIZE],
	input_lock: i8,
}

// Opcode
fn get_op(op: u16) -> u16 {
	(op & 0xF000) >> 12
}

// A 12-bit value, the lowest 12 bits of the instruction
fn get_addr(op: u16) -> u16 {
	op & 0x0FFF
}

// A 4-bit value, the lowest 4 bits of the instruction
fn get_nibble(op: u16) -> u16 {
	op & 0x000F
}

// A 4-bit value, the lower 4 bits of the high byte of the instruction
fn get_4bit_h(op: u16) -> u16 {
	(op & 0x0F00) >> 8
}

// A 4-bit value, the upper 4 bits of the low byte of the instruction
fn get_4bit_l(op: u16) -> u16 {
	(op & 0x00F0) >> 4
}
// An 8-bit value, the lowest 8 bits of the instruction
fn get_byte(op: u16) -> u16 {
	op & 0x00FF
}

#[derive(Debug)]
enum EvalError {
	UnknownOpcode,
}

impl CHIP8 {
	pub fn new() -> CHIP8 {

		CHIP8 {
			vx: [0; 16],
			i: 0, pc: 0,
			stack: [0; STACK_SIZE], sp: 0,
			dt: 0,
			st: 0,
			ram: [0; MEM_SIZE],
			display: [[0; DISPLAY_MAX_Y]; DISPLAY_MAX_X],
			keyboard: [false; KEYBOARD_SIZE],
			input_lock: -1
		}
	}

	fn init(&mut self) {

		let mut addr = 0x0;

		// load sprites
		for sprite in [
			[ 0xf0, 0x90, 0x90, 0x90, 0xf0 ], // 0
			[ 0x20, 0x60, 0x20, 0x20, 0x70 ], // 1
			[ 0xf0, 0x10, 0xf0, 0x80, 0xf0 ], // 2
			[ 0xf0, 0x10, 0xf0, 0x10, 0xf0 ], // 3
			[ 0x90, 0x90, 0xf0, 0x10, 0x10 ], // 4
			[ 0xf0, 0x80, 0xf0, 0x10, 0xf0 ], // 5
			[ 0xf0, 0x80, 0xf0, 0x90, 0xf0 ], // 6
			[ 0xf0, 0x10, 0x20, 0x40, 0x40 ], // 7
			[ 0xf0, 0x90, 0xf0, 0x90, 0xf0 ], // 8
			[ 0xf0, 0x90, 0xf0, 0x10, 0xf0 ], // 9
			[ 0xf0, 0x90, 0xf0, 0x90, 0x90 ], // a
			[ 0xe0, 0x90, 0xe0, 0x90, 0xe0 ], // b
			[ 0xf0, 0x80, 0x80, 0x80, 0xf0 ], // c
			[ 0xe0, 0x90, 0x90, 0x90, 0xe0 ], // d
			[ 0xf0, 0x80, 0xf0, 0x80, 0xf0 ], // e
			[ 0xf0, 0x80, 0xf0, 0x80, 0x80 ], // f
		].iter() {
			for val in sprite {
				self.ram[addr] = *val;
				addr += 1;
			}
		}

	}

	fn lrom(&mut self, rom_path: &String, offset: usize) {

		let mut rom_file = File::open(rom_path).unwrap();
		let mut i: usize = 0;
		'rom_read: loop {
			let inst: u16 = match rom_file.read_u16::<BigEndian>() {
				Ok(res) => res,
				Err(_e) => break 'rom_read,
			};
			let idx = i + offset;

			self.ram[idx]     = (inst >> 8) as u8;
			self.ram[idx + 1] = (inst & 0xFF) as u8;

			i += 2
		}
	}

	fn step(&mut self) -> Result<(), EvalError> {

		print!("pc [{:x?}]: ", self.pc);

		let inst = 
			((self.ram[self.pc as usize] as u16) << 8) | 
			self.ram[(self.pc as usize) + 1] as u16;

		let op = get_op(inst);

		let mut increment_after = true;

		match op {

			0x0 => {

				if inst == 0x00e0 {

					self.display = [[0; DISPLAY_MAX_Y]; DISPLAY_MAX_X];

					println!("cls");

				} else if inst == 0x00ee {

					self.pc = self.stack[self.sp as usize];
					self.sp -= 1;

					increment_after = false;

					println!("ret");

				} else {

					let addr = get_addr(inst);

					self.pc = addr;

					increment_after = false;

					println!("sys {:x?}", addr);

				}

			}

			0x1 => {

				let addr = get_addr(inst);

				self.pc = addr;

				increment_after = false;

				println!("jp {:x?}", addr);

			}

			0x2 => {

				let addr = get_addr(inst);

				self.sp += 1;
				self.stack[self.sp as usize] = self.pc;

				self.pc = addr;
				
				increment_after = false;

				println!("call {:x?}", addr);

			}

			0x3 => {

				let reg = get_4bit_h(inst);
				let lit = get_byte(inst);

				if self.vx[reg as usize] == lit {

					self.pc += 2;

				}

				println!("se v{:x?} {:x?}", reg, lit);

			}

			0x4 => {

				let reg = get_4bit_h(inst);
				let lit = get_byte(inst);

				if self.vx[reg as usize] != lit {

					self.pc += 2;

				}

				println!("sne v{:x?} {:x?}", reg, lit);

			}

			0x5 => {

				let regx = get_4bit_h(inst);
				let regy = get_4bit_l(inst);

				if self.vx[regx as usize] == self.vx[regy as usize] {

					self.pc += 2;

				}

				println!("se v{:x?} v{:x?}", regx, regy);

			}

			0x6 => {

				let reg = get_4bit_h(inst) as usize;
				let lit = get_byte(inst);

				self.vx[reg] = lit;

				println!("ld v{:x?} {:x?}", reg, lit);

			}

			0x7 => {

				let reg = get_4bit_h(inst);
				let lit = get_byte(inst);

				self.vx[reg as usize] += lit;

				println!("add v{:x?} {:x?}", reg, lit);

			}

			0x9 => {

				let regx = get_4bit_h(inst);
				let regy = get_4bit_l(inst);

				if self.vx[regx as usize] != self.vx[regy as usize] {

					self.pc += 2;

				}

				println!("sne v{:x?} v{:x?}", regx, regy);

			}

			0xa => {

				let addr = get_addr(inst);

				self.i = addr;

				println!("ld i {:x?}", addr);

			}

			0xc => {

				let mut rng = rand::thread_rng();
				let n = rng.gen_range(0, 255);
				let reg = get_4bit_h(inst) as usize;
				let lit = get_byte(inst);

				self.vx[reg] = lit & n;

				println!("rnd v{:x?} {:x?}", reg, lit);

			}

			0xd => {

				let regx = get_4bit_h(inst);
				let regy = get_4bit_l(inst);
				let nibble = get_nibble(inst);

				self.vx[0xf] = 0;

				let sprite = &self.ram[
					self.i as usize .. (self.i + nibble) as usize
					];

				let mut x_offset = 0;
				let mut y_offset = 0;
				for val in sprite {

					for i in 0 .. 8 {

						let prev = self.display
							[(regx + x_offset) as usize]
							[(regy + y_offset) as usize];

						let cur = prev ^ ((val  >> i) & 0x1 as u8);

						self.display
							[(regx + x_offset) as usize]
							[(regy + y_offset) as usize] = cur;

						x_offset += 1;

						if x_offset == DISPLAY_MAX_X as u16 {
							y_offset += 1;
							x_offset = 0;

							if y_offset == DISPLAY_MAX_Y as u16 {
								y_offset = 0;
							}
						}

						if prev == 1 && cur == 0 {
							self.vx[0xf] = 1;
						}

					}

					y_offset += 1;
					x_offset = 0;

					if y_offset == DISPLAY_MAX_Y as u16 {
						y_offset = 0;
					}
				}

				println!("drw v{:x?} v{:x?} {:x?}", regx, regy, nibble);
			}

			0xe => {

				let reg = get_4bit_h(inst);
				let lit = get_byte(inst);

				if lit == 0x9e {
					if self.keyboard[self.vx[reg as usize] as usize] {
						self.pc += 2;
					}
				} else if lit == 0xa1 {
					if !self.keyboard[self.vx[reg as usize] as usize] {
						self.pc += 2;
					}
				} else {

					println!(
						"pc: {:x?}, opcode: {:x?}, {:x?}",
						self.pc,
						op,
						inst
						);
				
					return Err(EvalError::UnknownOpcode)
				}

			}

			0xf => {

				let lit = get_byte(inst);
				let reg = get_4bit_h(inst);

				if lit == 0x33 {

					let val = self.vx[reg as usize];

					let hundreds = ((val / 100) % 100) as u8;
					let tens = ((val / 10) % 10) as u8;
					let ones = (val % 10) as u8;

					self.ram[self.i as usize] = hundreds;
					self.ram[(self.i+1) as usize] = tens;
					self.ram[(self.i+2) as usize] = ones;

					println!("ld b v{:x?}", reg);

				} else if lit == 0x0a {

					self.input_lock = reg as i8;

					println!("ld b v{:x?}", reg);

				} else if lit == 0x29 {

					self.i = reg * 5;

					println!("ld f v{:x?}", reg);

				} else if lit == 0x65 {

					for i in 0 .. reg {
						self.vx[i as usize] = self.ram[self.i as usize] as u16;
						self.i += 1;
					}

					println!("ld b v{:x?} [i]", reg);

				} else {

					println!(
						"pc: {:x?}, opcode: {:x?}, {:x?}",
						self.pc,
						op,
						inst
						);
				
					return Err(EvalError::UnknownOpcode)
				}

			}

			_ => {

				println!(
					"pc: {:x?}, opcode: {:x?}, {:x?}",
					self.pc,
					op,
					inst
					);
				
				return Err(EvalError::UnknownOpcode)
			}

		}

		if increment_after {
			self.pc += 2;
		}

		Ok(())

	}

	fn check_lock(&mut self, key: u16) {
		println!("check lock: {:x?}, L: {:x?}", key, self.input_lock);
        if self.input_lock >= 0 {
        	self.vx[self.input_lock as usize] = key;
        	self.input_lock = -1
        }
	}
}

fn main() {

	let args: Vec<String> = env::args().collect();

	let mut vm: CHIP8 = CHIP8::new();
	vm.init();

	vm.lrom(&args[1], 0x200);
	vm.pc = 0x200;


	// sdl init
	let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
 
    let window = video_subsystem.window(
    	"CHIP-8 Emulator",
    	(DISPLAY_MAX_X * DISPLAY_PIXEL) as u32,
    	(DISPLAY_MAX_Y * DISPLAY_PIXEL) as u32
    	)
        .position_centered()
        .build()
        .unwrap();
 
    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut dt_timer = Instant::now();
    let mut st_timer = Instant::now();
    let time_dt_ms = 1000 / DELAY_TIMER_HZ as u128;
    let time_st_ms = 1000 / SOUND_TIMER_HZ as u128;

	'mainloop: loop {

		canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'mainloop
                },
                Event::KeyDown { keycode: Some(Keycode::Num1), .. } => {
                    vm.keyboard[0x1] = true;
                    vm.check_lock(0x1);
                },
                Event::KeyDown { keycode: Some(Keycode::Num2), .. } => {
                    vm.keyboard[0x2] = true;
                    vm.check_lock(0x2);
                },
                Event::KeyDown { keycode: Some(Keycode::Num3), .. } => {
                    vm.keyboard[0x3] = true;
                    vm.check_lock(0x3);
                },
                Event::KeyDown { keycode: Some(Keycode::Num4), .. } => {
                    vm.keyboard[0xc] = true;
                    vm.check_lock(0xc);
                },
                Event::KeyDown { keycode: Some(Keycode::Q), .. } => {
                    vm.keyboard[0x4] = true;
                    vm.check_lock(0x4);
                },
                Event::KeyDown { keycode: Some(Keycode::W), .. } => {
                    vm.keyboard[0x5] = true;
                    vm.check_lock(0x5);
                },
                Event::KeyDown { keycode: Some(Keycode::E), .. } => {
                    vm.keyboard[0x6] = true;
                    vm.check_lock(0x6);
                },
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    vm.keyboard[0xd] = true;
                    vm.check_lock(0xd);
                },
                Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                    vm.keyboard[0x7] = true;
                    vm.check_lock(0x7);
                },
                Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                    vm.keyboard[0x8] = true;
                    vm.check_lock(0x8);
                },
                Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                    vm.keyboard[0x9] = true;
                    vm.check_lock(0x9);
                },
                Event::KeyDown { keycode: Some(Keycode::F), .. } => {
                    vm.keyboard[0xe] = true;
                    vm.check_lock(0xe);
                },
                Event::KeyDown { keycode: Some(Keycode::Z), .. } => {
                    vm.keyboard[0xa] = true;
                    vm.check_lock(0xa);
                },
                Event::KeyDown { keycode: Some(Keycode::X), .. } => {
                    vm.keyboard[0x0] = true;
                    vm.check_lock(0x0);
                },
                Event::KeyDown { keycode: Some(Keycode::C), .. } => {
                    vm.keyboard[0xb] = true;
                    vm.check_lock(0xb);
                },
                Event::KeyDown { keycode: Some(Keycode::V), .. } => {
                    vm.keyboard[0xf] = true;
                    vm.check_lock(0xf);
                },
                Event::KeyUp { keycode: Some(Keycode::Num1), .. } => {
                    vm.keyboard[0x1] = false
                },
                Event::KeyUp { keycode: Some(Keycode::Num2), .. } => {
                    vm.keyboard[0x2] = false
                },
                Event::KeyUp { keycode: Some(Keycode::Num3), .. } => {
                    vm.keyboard[0x3] = false
                },
                Event::KeyUp { keycode: Some(Keycode::Num4), .. } => {
                    vm.keyboard[0xc] = false
                },
                Event::KeyUp { keycode: Some(Keycode::Q), .. } => {
                    vm.keyboard[0x4] = false
                },
                Event::KeyUp { keycode: Some(Keycode::W), .. } => {
                    vm.keyboard[0x5] = false
                },
                Event::KeyUp { keycode: Some(Keycode::E), .. } => {
                    vm.keyboard[0x6] = false
                },
                Event::KeyUp { keycode: Some(Keycode::R), .. } => {
                    vm.keyboard[0xd] = false
                },
                Event::KeyUp { keycode: Some(Keycode::A), .. } => {
                    vm.keyboard[0x7] = false
                },
                Event::KeyUp { keycode: Some(Keycode::S), .. } => {
                    vm.keyboard[0x8] = false
                },
                Event::KeyUp { keycode: Some(Keycode::D), .. } => {
                    vm.keyboard[0x9] = false
                },
                Event::KeyUp { keycode: Some(Keycode::F), .. } => {
                    vm.keyboard[0xe] = false
                },
                Event::KeyUp { keycode: Some(Keycode::Z), .. } => {
                    vm.keyboard[0xa] = false
                },
                Event::KeyUp { keycode: Some(Keycode::X), .. } => {
                    vm.keyboard[0x0] = false
                },
                Event::KeyUp { keycode: Some(Keycode::C), .. } => {
                    vm.keyboard[0xb] = false
                },
                Event::KeyUp { keycode: Some(Keycode::V), .. } => {
                    vm.keyboard[0xf] = false
                },
                _ => {}
            }
        }

        if dt_timer.elapsed().as_millis() > time_dt_ms {
        	dt_timer = Instant::now();
        	if vm.dt > 0 {
        		vm.dt -= 1;
        	}
        }

        if st_timer.elapsed().as_millis() > time_st_ms {
        	st_timer = Instant::now();
        	if vm.st > 0 {
        		vm.st -= 1;
        	}
        }

        if vm.input_lock < 0 {
	        match vm.step() {
				Ok(_) => {},
				Err(_) => break 'mainloop ,
			}
		}

		for y in 0 .. DISPLAY_MAX_Y {
			for x in 0 .. DISPLAY_MAX_X {
				if vm.display[x][y] == 0 {
					canvas.set_draw_color(COLOR_BLACK);
				} else {
					canvas.set_draw_color(COLOR_WHITE);
				}
				canvas.fill_rect(
					Rect::new(
						(x * DISPLAY_PIXEL) as i32, (y * DISPLAY_PIXEL) as i32,
						DISPLAY_PIXEL as u32, DISPLAY_PIXEL as u32
						)
					).unwrap();
			}
		}

        canvas.present();

        std::thread::sleep(std::time::Duration::from_millis(100));

	}

}
