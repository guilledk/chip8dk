// http://devernay.free.fr/hacks/chip8/C8TECH10.HTM

extern crate rand;
extern crate byteorder;




use std::fs::File;
use std::env;

use rand::Rng;

use byteorder::{BigEndian, ReadBytesExt};

const STACK_SIZE: usize = 256;
const MEM_SIZE: usize = 4096;

struct CHIP8 {
	vx: [u16; 16], // V registers
	i: u16, pc: u16, // I & program counter
	stack: [u8; STACK_SIZE], sp: u8,
	delay_timer: u8,
	sound_timer: u8,
	ram: [u8; MEM_SIZE]
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
			delay_timer: 0,
			sound_timer: 0,
			ram: [0; MEM_SIZE]
		}
	}

	fn lrom(&mut self, rom_path: &String, offset: usize) {

		let mut rom_file = File::open(rom_path).unwrap();

		let file_meta = rom_file.metadata().unwrap();

		for i in (0 .. file_meta.len()).step_by(2) {
			let inst: u16 = rom_file.read_u16::<BigEndian>().unwrap();
			let idx: usize = i as usize + offset;

			self.ram[idx]     = (inst >> 8) as u8;
			self.ram[idx + 1] = (inst & 0xFF) as u8;
		}
	}

	fn step(&mut self) -> Result<(), EvalError> {

		let inst = 
			((self.ram[self.pc as usize] as u16) << 8) | 
			self.ram[(self.pc as usize) + 1] as u16;

		let op = get_op(inst);

		if op == 0x0 {

			if inst == 0x00e0 {

				println!("cls");

			} else if inst == 0x00ee {

				println!("ret");

			} else {

				let addr = get_addr(inst);

				self.pc = addr;

				println!("sys {:x?}", addr);

			}

		} else if op == 0x6 {

			let reg = get_4bit_h(inst) as usize;
			let lit = get_byte(inst);

			self.vx[reg] = lit;

			println!("ld v{:x?} {:x?}", reg, lit);

		}  else if op == 0xc {

			let mut rng = rand::thread_rng();
			let n = rng.gen_range(0, 255);
			let reg = get_4bit_h(inst) as usize;
			let lit = get_byte(inst);

			self.vx[reg] = lit & n;

			println!("rnd v{:x?} {:x?}", reg, lit);

		} else {

			println!(
				"pc: {:x?}, opcode: {:x?}, {:x?}",
				self.pc,
				op,
				inst
				);
			
			return Err(EvalError::UnknownOpcode)

		};

		Ok(())

	}
}

fn main() {

	let args: Vec<String> = env::args().collect();

	let mut vm = CHIP8::new();

	vm.lrom(&args[1], 0x200);
	vm.pc = 0x200;

	let mut exit = false;
	while !exit {

		match vm.step() {
			Ok(_) => vm.pc += 2,
			Err(_) => exit = true,
		}

	}

}
