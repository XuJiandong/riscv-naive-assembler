use clap::{App, Arg};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::{fmt, io};

lazy_static! {
    pub static ref REG_MAP: HashMap<String, u8> = {
        let mut map = HashMap::new();
        map.insert(String::from("zero"), 0);
        map.insert(String::from("ra"), 1);
        map.insert(String::from("sp"), 2);
        map.insert(String::from("gp"), 3);
        map.insert(String::from("tp"), 4);
        map.insert(String::from("t0"), 5);
        map.insert(String::from("t1"), 6);
        map.insert(String::from("t2"), 7);
        map.insert(String::from("s0"), 8); // s0 == fp
        map.insert(String::from("fp"), 8); // s0 == fp
        map.insert(String::from("s1"), 9);
        map.insert(String::from("a0"), 10);
        map.insert(String::from("a1"), 11);
        map.insert(String::from("a2"), 12);
        map.insert(String::from("a3"), 13);
        map.insert(String::from("a4"), 14);
        map.insert(String::from("a5"), 15);
        map.insert(String::from("a6"), 16);
        map.insert(String::from("a7"), 17);
        map.insert(String::from("s2"), 18);
        map.insert(String::from("s3"), 19);
        map.insert(String::from("s4"), 20);
        map.insert(String::from("s5"), 21);
        map.insert(String::from("s6"), 22);
        map.insert(String::from("s7"), 23);
        map.insert(String::from("s8"), 24);
        map.insert(String::from("s9"), 25);
        map.insert(String::from("s10"), 26);
        map.insert(String::from("s11"), 27);
        map.insert(String::from("t3"), 28);
        map.insert(String::from("t4"), 29);
        map.insert(String::from("t5"), 30);
        map.insert(String::from("t6"), 31);
        map
    };
}

fn reg_name2value(name: &str) -> u8 {
    let res = REG_MAP.get(name);
    if res.is_none() {
        panic!("can't find register name {}", name);
    }
    res.unwrap().clone()
}

struct BinaryInstruction {
    pub data: [u8; 4],
    flag_shamt: bool,
    flag_funct6: bool,
}

impl fmt::Display for BinaryInstruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.flag_shamt {
            if !self.flag_funct6 {
                panic!("funct6 should be paired with shamt");
            }
        } else {
            if self.flag_funct6 {
                panic!("funct6 should be paired with shamt");
            }
        }

        write!(
            f,
            ".byte 0x{:02x},0x{:02x},0x{:02x},0x{:02x}",
            self.data[0], self.data[1], self.data[2], self.data[3]
        )
    }
}

impl BinaryInstruction {
    fn new() -> BinaryInstruction {
        BinaryInstruction {
            data: [0; 4],
            flag_shamt: false,
            flag_funct6: false,
        }
    }
    fn to_bits_string(&self) -> String {
        fn dump(bits: Vec<u8>) -> String {
            let str: Vec<String> = bits.into_iter().map(|i| format!("{}", i)).rev().collect();
            str.join("")
        }
        let opcode = self.get(0, 6);
        let rd = self.get(7, 11);
        let funct3 = self.get(12, 14);
        let rs1 = self.get(15, 19);
        let rs2 = self.get(20, 24);
        let funct7 = self.get(25, 31);
        format!(
            "funct7: {} rs2: {} rs1: {} funct3: {} rd: {} opcode: {}",
            dump(funct7),
            dump(rs2),
            dump(rs1),
            dump(funct3),
            dump(rd),
            dump(opcode)
        )
    }
    fn bits_array(val: u8, count: usize) -> Vec<u8> {
        assert_eq!(val >> count, 0);
        let mut res = Vec::<u8>::new();
        for index in 0..count {
            if (val & (1 << index)) > 0 {
                res.push(1);
            } else {
                res.push(0);
            }
        }

        res
    }

    // set bits at positions from [begin, end], inclusively.
    fn set(&mut self, begin: u8, end: u8, bits: Vec<u8>) {
        let begin = begin as usize;
        let end = end as usize;

        assert_eq!((end - begin + 1) as usize, bits.len());
        for index in begin..=end {
            let byte_index = index / 8;
            let bit_index = index % 8;
            let index2 = index - begin;
            assert!(bits[index2] == 0 || bits[index2] == 1);
            if bits[index2] == 0 {
                self.data[byte_index] &= !(1 << bit_index as u8); // clear
            } else {
                self.data[byte_index] |= 1 << bit_index as u8; // set
            }
        }
    }
    fn get(&self, begin: u8, end: u8) -> Vec<u8> {
        let mut res = Vec::<u8>::new();
        let begin = begin as usize;
        let end = end as usize;

        for index in begin..=end {
            let byte_index = index / 8;
            let bit_index = index % 8;

            if (self.data[byte_index] & (1 << bit_index)) > 0 {
                res.push(1);
            } else {
                res.push(0);
            }
        }
        assert_eq!((end - begin + 1) as usize, res.len());
        res
    }
    fn set_opcode(&mut self, opcode: u8) {
        let bits = BinaryInstruction::bits_array(opcode, 7);
        self.set(0, 6, bits);
    }
    fn set_rd(&mut self, rd: &str) {
        let rd = reg_name2value(rd);
        let bits = BinaryInstruction::bits_array(rd, 5);
        self.set(7, 11, bits);
    }
    fn set_funct3(&mut self, funct3: u8) {
        let bits = BinaryInstruction::bits_array(funct3, 3);
        self.set(12, 14, bits);
    }
    fn set_rs1(&mut self, rs1: &str) {
        let rs1 = reg_name2value(rs1);
        let bits = BinaryInstruction::bits_array(rs1, 5);
        self.set(15, 19, bits);
    }
    fn set_rs2(&mut self, rs2: &str) {
        let rs2 = reg_name2value(rs2);
        let bits = BinaryInstruction::bits_array(rs2, 5);
        self.set(20, 24, bits);
    }

    fn set_shamt(&mut self, shamt: u8) {
        let bits = BinaryInstruction::bits_array(shamt, 6);
        self.set(20, 25, bits);
        self.flag_shamt = true;
    }
    // funct6 <-> shamt
    fn set_funct6(&mut self, funct6: u8) {
        let bits = BinaryInstruction::bits_array(funct6, 6);
        self.set(26, 31, bits);
        self.flag_funct6 = true;
    }

    fn set_funct7(&mut self, funct7: u8) {
        let bits = BinaryInstruction::bits_array(funct7, 7);
        self.set(25, 31, bits);
    }
    fn set_operands(&mut self, operands: &Vec<String>) {
        assert_eq!(operands.len(), 3);

        self.set_rd(operands[0].as_ref());
        self.set_rs1(operands[1].as_ref());
        self.set_rs2(operands[2].as_ref());
    }
    fn set_2operands(&mut self, operands: &Vec<String>, rs2: u8) {
        assert_eq!(operands.len(), 2);

        self.set_rd(operands[0].as_ref());
        self.set_rs1(operands[1].as_ref());

        let bits = BinaryInstruction::bits_array(rs2, 5);
        self.set(20, 24, bits);
    }

    fn set_immediate(&mut self, operands: &Vec<String>) {
        self.set_rd(operands[0].as_ref());
        self.set_rs1(operands[1].as_ref());
        let shamt = operands[2].parse::<u8>().unwrap();
        self.set_shamt(shamt);
    }
}

struct TextInstruction {
    pub opcode: String,
    pub operands: Vec<String>,
    pub raw: Option<String>,
}

impl fmt::Display for TextInstruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.opcode, self.operands.join(","))
    }
}

impl TextInstruction {
    fn new() -> TextInstruction {
        TextInstruction {
            opcode: Default::default(),
            operands: Default::default(),
            raw: None,
        }
    }
    fn convert(&self) -> Option<BinaryInstruction> {
        let mut res = BinaryInstruction::new();
        match self.opcode.as_ref() {
            "add.uw" => {
                res.set_opcode(0b0111011);
                res.set_funct3(0);
                res.set_funct7(0b0000100);
                res.set_operands(&self.operands);
                Some(res)
            }
            "andn" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b111);
                res.set_funct7(0b0100000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "bclr" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b001);
                res.set_funct7(0b0100100);
                res.set_operands(&self.operands);
                Some(res)
            }
            "bclri" => {
                res.set_opcode(0b0010011);
                res.set_funct3(0b001);
                res.set_funct6(0b010010);
                res.set_immediate(&self.operands);
                Some(res)
            }
            "bext" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b101);
                res.set_funct7(0b0100100);
                res.set_operands(&self.operands);
                Some(res)
            }
            "bexti" => {
                res.set_opcode(0b0010011);
                res.set_funct3(0b101);
                res.set_funct6(0b010010);
                res.set_immediate(&self.operands);
                Some(res)
            }
            "binv" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b001);
                res.set_funct7(0b0110100);
                res.set_operands(&self.operands);
                Some(res)
            }
            "binvi" => {
                res.set_opcode(0b0010011);
                res.set_funct3(0b001);
                res.set_funct6(0b011010);
                res.set_immediate(&self.operands);
                Some(res)
            }
            "bset" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b001);
                res.set_funct7(0b0010100);
                res.set_operands(&self.operands);
                Some(res)
            }
            "bseti" => {
                res.set_opcode(0b0010011);
                res.set_funct3(0b001);
                res.set_funct6(0b001010);
                res.set_immediate(&self.operands);
                Some(res)
            }
            "clmul" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b001);
                res.set_funct7(0b0000101);
                res.set_operands(&self.operands);
                Some(res)
            }
            "clmulh" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b011);
                res.set_funct7(0b0000101);
                res.set_operands(&self.operands);
                Some(res)
            }
            "clmulr" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b010);
                res.set_funct7(0b0000101);
                res.set_operands(&self.operands);
                Some(res)
            }
            "clz" => {
                res.set_opcode(0b0010011);
                res.set_funct3(0b001);
                res.set_funct7(0b0110000);
                res.set_2operands(&self.operands, 0);
                Some(res)
            }
            "clzw" => {
                res.set_opcode(0b0011011);
                res.set_funct3(0b001);
                res.set_funct7(0b0110000);
                res.set_2operands(&self.operands, 0);
                Some(res)
            }
            "cpop" => {
                res.set_opcode(0b0010011);
                res.set_funct3(0b001);
                res.set_funct7(0b0110000);
                res.set_2operands(&self.operands, 0b00010);
                Some(res)
            }
            "cpopw" => {
                res.set_opcode(0b0011011);
                res.set_funct3(0b001);
                res.set_funct7(0b0110000);
                res.set_2operands(&self.operands, 0b00010);
                Some(res)
            }
            "ctz" => {
                res.set_opcode(0b0010011);
                res.set_funct3(0b001);
                res.set_funct7(0b0110000);
                res.set_2operands(&self.operands, 0b00001);
                Some(res)
            }
            "ctzw" => {
                res.set_opcode(0b0011011);
                res.set_funct3(0b001);
                res.set_funct7(0b0110000);
                res.set_2operands(&self.operands, 0b00001);
                Some(res)
            }
            "max" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b110);
                res.set_funct7(0b0000101);
                res.set_operands(&self.operands);
                Some(res)
            }
            "maxu" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b111);
                res.set_funct7(0b0000101);
                res.set_operands(&self.operands);
                Some(res)
            }
            "min" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b100);
                res.set_funct7(0b0000101);
                res.set_operands(&self.operands);
                Some(res)
            }
            "minu" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b101);
                res.set_funct7(0b0000101);
                res.set_operands(&self.operands);
                Some(res)
            }
            "orc.b" => {
                res.set_opcode(0b0010011);
                res.set_funct3(0b101);
                res.set_funct7(0b0010100);
                res.set_2operands(&self.operands, 0b00111);
                Some(res)
            }
            "orn" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b110);
                res.set_funct7(0b0100000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "rev8" => {
                res.set_opcode(0b0010011);
                res.set_funct3(0b101);
                res.set_funct7(0b0110101);
                res.set_2operands(&self.operands, 0b11000);
                Some(res)
            }
            "rol" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b001);
                res.set_funct7(0b0110000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "rolw" => {
                res.set_opcode(0b0111011);
                res.set_funct3(0b001);
                res.set_funct7(0b0110000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "ror" => {
                res.set_opcode(0b110011);
                res.set_funct3(0b101);
                res.set_funct7(0b0110000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "rori" => {
                res.set_opcode(0b0010011);
                res.set_funct3(0b101);
                res.set_funct6(0b011000);
                res.set_immediate(&self.operands);
                Some(res)
            }
            "roriw" => {
                res.set_opcode(0b0011011);
                res.set_funct3(0b101);
                res.set_funct7(0b0110000);

                let shamt = self.operands[2].parse::<u8>().unwrap();
                let mut operands = self.operands.clone();
                operands.pop();
                res.set_2operands(&operands, shamt);
                Some(res)
            }
            "rorw" => {
                res.set_opcode(0b0111011);
                res.set_funct3(0b101);
                res.set_funct7(0b0110000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "sext.b" => {
                res.set_opcode(0b0010011);
                res.set_funct3(0b001);
                res.set_funct7(0b0110000);
                res.set_2operands(&self.operands, 0b00100);
                Some(res)
            }
            "sext.h" => {
                res.set_opcode(0b0010011);
                res.set_funct3(0b001);
                res.set_funct7(0b0110000);
                res.set_2operands(&self.operands, 0b00101);
                Some(res)
            }
            "sh1add" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b010);
                res.set_funct7(0b0010000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "sh1add.uw" => {
                res.set_opcode(0b0111011);
                res.set_funct3(0b010);
                res.set_funct7(0b0010000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "sh2add" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b100);
                res.set_funct7(0b0010000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "sh2add.uw" => {
                res.set_opcode(0b0111011);
                res.set_funct3(0b100);
                res.set_funct7(0b0010000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "sh3add" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b110);
                res.set_funct7(0b0010000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "sh3add.uw" => {
                res.set_opcode(0b0111011);
                res.set_funct3(0b110);
                res.set_funct7(0b0010000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "slli.uw" => {
                res.set_opcode(0b0011011);
                res.set_funct3(0b001);
                res.set_funct6(0b000010);
                res.set_immediate(&self.operands);
                Some(res)
            }
            "xnor" => {
                res.set_opcode(0b0110011);
                res.set_funct3(0b100);
                res.set_funct7(0b0100000);
                res.set_operands(&self.operands);
                Some(res)
            }
            "zext.h" => {
                res.set_opcode(0b0111011);
                res.set_funct3(0b100);
                res.set_funct7(0b0000100);
                res.set_2operands(&self.operands, 0b00000);
                Some(res)
            }
            _ => None,
        }
    }
}

fn parse_line(line: &str) -> TextInstruction {
    let fields: Vec<&str> = line.split(" ").collect();
    if fields.len() >= 2 {
        let opcode = String::from(fields[0]);
        let index = line.find(" ").unwrap();
        let operands: Vec<String> = line[index + 1..]
            .split(",")
            .map(|r| String::from(r.trim()))
            .filter(|r| r.len() > 0)
            .collect();
        TextInstruction {
            opcode,
            operands,
            raw: None,
        }
    } else {
        let mut r = TextInstruction::new();
        r.raw = Some(String::from(line));
        r
    }
}

fn test(line: &str, bytes: &str) {
    let inst = parse_line(line);
    let inst2 = inst.convert();
    if let Some(i) = inst2 {
        let res = i.to_string();
        // println!("{}\n{}", line, i.to_bits_string());
        assert_eq!(bytes, res);
    } else {
        assert_eq!(format!("{}", inst), bytes);
    }
}

#[test]
fn test_adduw() {
    test("add.uw a2, s11, s5", ".byte 0x3b,0x86,0x5d,0x09");
}

#[test]
fn test_andn() {
    test("andn zero, tp, s6", ".byte 0x33,0x70,0x62,0x41");
}

#[test]
fn test_bclr() {
    test("bclr s10, a4, a5", ".byte 0x33,0x1d,0xf7,0x48");
}

#[test]
fn test_others() {
    test("sh3add.uw a3,s5,gp", ".byte 0xbb,0xe6,0x3a,0x20");
}

#[test]
fn test_add() {
    test("add t6, t6, s0", "add t6,t6,s0");
    test("xor t6, t6, s6", "xor t6,t6,s6");
}

fn main() {
    let matches = App::new("rna")
        .version("1.0")
        .about("A naive assembler for RISC-V")
        .arg(
            Arg::with_name("input")
                .required(false)
                .short("i")
                .long("input")
                .takes_value(true)
                .help("input file, default stdin"),
        )
        .arg(
            Arg::with_name("debug")
                .required(false)
                .short("d")
                .long("debug")
                .help("debug flags, print more information: encoding"),
        )
        .get_matches();
    let mut content = String::new();
    let is_debug = matches.is_present("debug");

    if matches.is_present("input") {
        let name = matches.value_of("input").unwrap();
        let mut input = File::open(name).unwrap();
        input.read_to_string(&mut content).unwrap();
    } else {
        let mut stdin = io::stdin();
        stdin.read_to_string(&mut content).unwrap();
    }
    let all_lines = content.split("\n");
    let all_lines: Vec<String> = all_lines
        .into_iter()
        .map(|l| l.trim())
        .map(|l| l.to_lowercase())
        .collect();
    let all_text_inst: Vec<TextInstruction> =
        all_lines.into_iter().map(|l| parse_line(&l)).collect();
    for inst in all_text_inst {
        if let Some(raw) = inst.raw {
            // unknown instruction, normally it's directive or label.
            println!("{}", raw);
        } else {
            if let Some(bin_inst) = inst.convert() {
                if is_debug {
                    println!("# Encoding {}", bin_inst.to_bits_string());
                }
                println!("# {}", inst);
                println!("{}", bin_inst);
            } else {
                // instruction, but not B-Extension
                println!("{}", inst);
            }
        }
    }
}
