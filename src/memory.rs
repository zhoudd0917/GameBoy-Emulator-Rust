use log::info;

use crate::{
    graphics::OAM_ADDRESS,
    utils::{address2string, bytes2word, Address, Byte, Word},
};

const MEMORY_SIZE: usize = 0x10000;
const BOOTROM_SIZE: usize = 0x100;
const ROM_SIZE: usize = 0x4000;

const DMA_ADDRESS: Address = 0xFF46;
const MBC_TYPE_ADDRESS: Address = 0x0147;
const ROM_SIZE_ADDRESS: Address = 0x0148;
const RAM_SIZE_ADDRESS: Address = 0x0149;

const UNLOAD_BOOT_ADDRESS: Address = 0xFF50;

#[derive(Debug, PartialEq, Eq)]
pub enum CartridgeType {
    None,
    RomOnly,
    MBC1,
    MBC3,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CartridgeState {
    None,
    RomOnly(RomState),
    MBC1(MBC1State),
    MBC3(MBC3State),
}

#[derive(Debug, PartialEq, Eq)]
pub struct RomState {}

#[derive(Debug, PartialEq, Eq)]
pub struct MBC1State {
    ram_enabled: bool,
    rom_number: usize,
    ram_number: usize,
}

impl MBC1State {
    fn new() -> Self {
        Self {
            rom_number: 1,
            ram_enabled: false,
            ram_number: 0,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MBC3State {
    ram_enabled: bool,
    rom_number: usize,
    ram_number: usize,
}

impl MBC3State {
    fn new() -> Self {
        Self {
            rom_number: 1,
            ram_enabled: false,
            ram_number: 0,
        }
    }
}

pub struct Memory {
    memory: [Byte; MEMORY_SIZE],
    boot_rom: [Byte; BOOTROM_SIZE],
    rom: Vec<Vec<Byte>>,
    #[allow(dead_code)]
    ram: Vec<Vec<Byte>>,
    cartridge: CartridgeState,
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            memory: [0; MEMORY_SIZE],
            boot_rom: [0; BOOTROM_SIZE],
            rom: Vec::new(),
            ram: Vec::new(),
            cartridge: CartridgeState::None,
        }
    }

    pub fn load_cartidge(&mut self, rom_data: Vec<u8>) {
        let ctype = self.get_cartridge_type_rom(&rom_data);
        let rom_size = self.get_rom_size_rom(&rom_data);
        let ram_size = self.get_ram_size_rom(&rom_data);
        info!("Load Rom Size {:#04X?}", rom_data.len(),);
        info!("Rom Type {:?}", ctype);
        info!("Rom Size {:?}", rom_size);
        info!("Ram Size {:?}", ram_size);

        self.cartridge = match ctype {
            CartridgeType::RomOnly => CartridgeState::RomOnly(RomState {}),
            CartridgeType::MBC1 => CartridgeState::MBC1(MBC1State::new()),
            CartridgeType::MBC3 => CartridgeState::MBC3(MBC3State::new()),
            CartridgeType::None => panic!("Unknown cartridge type"),
        };

        // copy rom_data to self.rom
        let rom_data = rom_data.as_slice();

        let rom_bank_num = 1 << (rom_size + 1);
        for i in 0..rom_bank_num {
            let mut rom_bank = Vec::with_capacity(ROM_SIZE);
            rom_bank.extend_from_slice(&rom_data[ROM_SIZE * i..ROM_SIZE * (i + 1)]);
            self.rom.push(rom_bank);
        }
        self.memory[BOOTROM_SIZE..ROM_SIZE].copy_from_slice(&self.rom[0][BOOTROM_SIZE..ROM_SIZE]);
        self.memory[ROM_SIZE..ROM_SIZE * 2].copy_from_slice(&self.rom[1]);
    }

    pub fn load_boot(&mut self, boot_data: Vec<u8>) {
        info!("Boot Size {:#04X?}", boot_data.len());
        self.boot_rom.copy_from_slice(&boot_data);
        self.memory[..BOOTROM_SIZE].copy_from_slice(&self.boot_rom);
    }

    pub fn read_byte(&self, address: Address) -> Byte {
        let address = address as usize;
        self.memory[address]
    }

    pub fn read_word(&self, address: Address) -> Word {
        let address = address as usize;
        bytes2word(self.memory[address], self.memory[address + 1])
    }

    /// Write byte to address according to MMU(Memory Management Unit)
    pub fn write_byte(&mut self, address: Address, byte: Byte) {
        match address {
            UNLOAD_BOOT_ADDRESS => self.unload_boot(),
            DMA_ADDRESS => self.dma(byte),
            _ => (),
        }

        let address = address as usize;

        let ctype = self.get_cartridge_type();
        match ctype {
            CartridgeType::RomOnly => {
                if address >= 0x8000 {
                    self.memory[address] = byte;
                }
            }
            CartridgeType::MBC1 => {
                if address >= 0x8000 {
                    self.memory[address] = byte;
                } else if address < 0x8000 {
                    unimplemented!("{}", address2string(address as Address));
                }
            }
            CartridgeType::MBC3 => {
                if address >= 0x8000 {
                    self.memory[address] = byte;
                } else if address < 0x8000 {
                    unimplemented!("{}", address2string(address as Address));
                }
            }
            CartridgeType::None => {
                self.memory[address] = byte;
            }
        }
    }

    /// Get cartridge type from memory
    pub fn get_cartridge_type(&self) -> CartridgeType {
        match self.cartridge {
            CartridgeState::None => CartridgeType::None,
            CartridgeState::RomOnly(_) => CartridgeType::RomOnly,
            CartridgeState::MBC1(_) => CartridgeType::MBC1,
            CartridgeState::MBC3(_) => CartridgeType::MBC3,
        }
    }

    /// Get cartridge type given rom (in vec)
    pub fn get_cartridge_type_rom(&self, rom: &[Byte]) -> CartridgeType {
        let rom_type = rom[MBC_TYPE_ADDRESS as usize];
        match rom_type {
            0x00 => CartridgeType::RomOnly,
            0x01 => CartridgeType::MBC1,
            0x13 => CartridgeType::MBC3,
            _ => unimplemented!("Rom type {:#04X?}", rom_type),
        }
    }

    /// Get rom size
    pub fn get_rom_size_rom(&self, rom: &[Byte]) -> usize {
        let rom_size = rom[ROM_SIZE_ADDRESS as usize].into();
        rom_size
    }

    /// Get ram size
    pub fn get_ram_size_rom(&self, rom: &[Byte]) -> usize {
        let ram_size = rom[RAM_SIZE_ADDRESS as usize].into();
        ram_size
    }

    fn unload_boot(&mut self) {
        info!("Unloading boot rom");
        self.memory[..BOOTROM_SIZE].copy_from_slice(&self.rom[0][..BOOTROM_SIZE]);
    }

    fn dma(&mut self, byte: Byte) {
        let size = 0x100;
        let src = bytes2word(0x00, byte) as usize;

        self.memory
            .copy_within(src..(src + size), OAM_ADDRESS as usize);
    }

    /// Wrapping add value to address
    pub fn wrapping_add(&mut self, address: Address, value: Byte) {
        assert!((address as usize) < MEMORY_SIZE);
        let mut mem_val = self.read_byte(address);
        mem_val = mem_val.wrapping_add(value);
        self.write_byte(address, mem_val);
    }

    pub fn write_test(&mut self, rom: Vec<Byte>) {
        self.memory[..rom.len()].copy_from_slice(&rom);
    }
}
