#![no_std]
#![no_main]

pub mod stm32_lib;

// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

extern crate alloc;

use alloc::vec::Vec;
use cortex_m_rt::entry;
use stm32_lib::rcc;
use stm32f4::stm32f446;

#[global_allocator]
static ALLOCATOR: emballoc::Allocator<4096> = emballoc::Allocator::new();

const SLAVE_ADDRESS: u8 = 0x4E >> 1;

pub fn i2c_start(i2c: &mut stm32f446::I2C1) {
    //send the Start condition
    i2c.cr1.modify(|_r, w| w.ack().set_bit()); //enabling the acknowledgement bit
    i2c.cr1.modify(|_r, w| w.start().set_bit());
    while !i2c.sr1.read().sb().is_start() { // waiting for the start condition
    }
}

fn i2c_write(data: u8, i2c: &mut stm32f446::I2C1) {
    //wait for the TXE bit 7 in cr1 to set. This indicates that the DR is empty
    while !i2c.sr1.read().tx_e().is_empty() {}
    //load the data in the data register
    i2c.dr.write(|w| w.dr().bits(data));
    //wait for data transfer to finish
    while !i2c.sr1.read().btf().is_finished() {}
}

fn i2c_address(address: u8, i2c: &mut stm32f446::I2C1) {
    //send the slave address to the DR register
    i2c.dr.write(|w| w.dr().bits(address));
    while !i2c.sr1.read().addr().is_match() {}
    let _temp = i2c.sr1.read(); //reading sr1 and sr2 to clear the addr bit
    let _temp2 = i2c.sr2.read();
}

fn i2c_stop(i2c: &mut stm32f446::I2C1) {
    i2c.cr1.write(|w| w.stop().set_bit());
}

fn i2c_write_multi(data: &mut Vec<u8>, size: &mut usize, i2c: &mut stm32f446::I2C1) {
    //wait for the TXE bit 7 in SR1 to set. This indicates that the DR is empty ;
    while !i2c.sr1.read().tx_e().is_empty() {}
    while *size > 0 {
        while !i2c.sr1.read().tx_e().is_empty() {}
        i2c.dr.modify(|_r, w| w.dr().bits(data[*size]));
        *size = *size - 1;
    }
    while !i2c.sr1.read().btf().is_finished() {}
}

//LCD functions

fn lcd_write(i2c: &mut stm32f446::I2C1, address: u8, data: Vec<u8>, size: usize) {
    i2c_start(i2c);
    i2c_address(address, i2c);
    for i in 0..size {
        i2c_write(data[i], i2c);
    }
    i2c_stop(i2c);
}

fn lcd_send_cmd(cmd: u8, i2c: &mut stm32f446::I2C1) {
    let data_u: u8;
    let data_l: u8;
    let temp: u8;
    let mut data: Vec<u8> = Vec::new();
    data_u = cmd & 0xf0;
    temp = cmd << 4;
    data_l = temp & 0xf0;
    data.push(data_u | 0x0c); //en = 1 & rs = 0
    data.insert(1, data_u | 0x08); //en = 0 & rs = 0
    data.insert(2, data_l | 0x0c); //en = 1 & rs = 0
    data.insert(3, data_l | 0x08); //en = 0 & rs = 0
    lcd_write(i2c, SLAVE_ADDRESS, data, 4);
}

fn lcd_send_data(data: u8, i2c: &mut stm32f446::I2C1) {
    let data_u: u8;
    let data_l: u8;
    let temp: u8;
    let mut data_t: Vec<u8> = Vec::new();
    data_u = data & 0xf0;
    temp = data << 4;
    data_l = temp & 0xf0;
    data_t.push(data_u | 0x0c); //en = 1 & rs = 1
    data_t.insert(1, data_u | 0x08); //en = 0 & rs = 1
    data_t.insert(2, data_l | 0x0c); //en = 1 & rs = 1
    data_t.insert(3, data_l | 0x08); //en = 1 & rs = 1
    lcd_write(i2c, SLAVE_ADDRESS, data_t, 4);
}

fn lcd_clear(i2c: &mut stm32f446::I2C1) {
    lcd_send_cmd(0x80, i2c);
    for _ in 0..70 {
        lcd_send_data(' ' as u8, i2c);
    }
}

fn lcd_put_cur(i2c: &mut stm32f446::I2C1, row: u8, col: &mut u8) {
    match row {
        0 => *col |= 0x80,
        1 => *col |= 0xc0,
        _ => {}
    }
    lcd_send_cmd(*col, i2c)
}

fn lcd_init(i2c: &mut stm32f446::I2C1, timer: &mut stm32f446::TIM11) {
    // 4 bit initialisation
    rcc::delay_ms(50, timer); //wait for >40ms
    lcd_send_cmd(0x30, i2c);
    rcc::delay_ms(5, timer); //wait for >4.1ms
    lcd_send_cmd(0x30, i2c);
    rcc::delay_us(150, timer); //wait for >100ms
    lcd_send_cmd(0x30, i2c);
    rcc::delay_ms(10, timer); //wait for >4.1ms
    lcd_send_cmd(0x20, i2c); //4bit mode
    rcc::delay_ms(10, timer);

    //display initialisation
    lcd_send_cmd(0x28, i2c); // function set ---> DL=0 (4 bit mode), N = 1 (2 line display) F = 0 (5*8 characters)
    rcc::delay_ms(1, timer);
    lcd_send_cmd(0x08, i2c); // Display on/off control --> D=0, C=0, B=0 ---> display off
    rcc::delay_ms(1, timer);
    lcd_send_cmd(0x01, i2c); // clear display
    rcc::delay_ms(2, timer);
    lcd_send_cmd(0x06, i2c); // Entry mode set --> I/D = 1 (increment cursor) & S = 0 (no shift)
    rcc::delay_ms(1, timer);
    lcd_send_cmd(0x0C, i2c); // Display on/off control --> D=1, C and B = 0. (Cursor and blink last two bits)
    rcc::delay_ms(1, timer);
}

fn lcd_write_str(i2c: &mut stm32f446::I2C1, input: &str) {
    //convert the string to a vector of bytes
    let bytes: Vec<u8> = input.bytes().collect();
    for &byte in &bytes {
        lcd_send_data(byte, i2c);
    }
}

#[entry]
fn main() -> ! {
    let peripherals = stm32f446::Peripherals::take().unwrap();
    let mut rcc = peripherals.RCC;
    let mut timer = peripherals.TIM11; //used for the delay
    let mut flash = peripherals.FLASH;
    let gpio = peripherals.GPIOB; //general purpose pin normally used as output.
    let mut pwr = peripherals.PWR;
    let mut i2c = peripherals.I2C1;
    let mut col: u8 = 0;

    rcc::initialize_clock(&mut rcc, &mut pwr, &mut flash);

    //Enable Timer clock
    rcc.apb2enr.modify(|_r, w| w.tim11en().set_bit());

    //initialize timer for delay
    // Set the prescaler and the ARR
    timer.psc.modify(|_r, w| w.psc().bits(0b0000000010110011)); //180MHz/180 = 1MHz ~ 1us, prescalar set to 179, ie. 179+1 = 180;
    timer.arr.modify(|_r, w| unsafe { w.arr().bits(0xffff) });

    //Enable the Timer, and wait for the update Flag to set
    timer.cr1.modify(|_r, w| w.cen().set_bit());
    timer.sr.read().uif().bit();
    while !timer.sr.read().uif().bit() {}

    //Enable 12c and gpio clocks
    rcc.apb1enr.write(|w| w.i2c1en().set_bit());
    rcc.ahb1enr.write(|w| w.gpioben().set_bit());

    //configure gpio pins

    gpio.moder.write(|w| w.moder8().bits(0b10)); // place pin PB8 and PB9 in alternate fucntion mode
    gpio.moder.modify(|_r, w| w.moder9().bits(0b10));

    gpio.otyper.modify(|_r, w| w.ot8().set_bit()); // enabling open drain mode for pin PB8 and PB9
    gpio.otyper.modify(|_r, w| w.ot9().set_bit());

    gpio.ospeedr.modify(|_r, w| w.ospeedr8().bits(0b11)); // setting the pins PB8 and PB9 to high speed (fastest)
    gpio.ospeedr.modify(|_r, w| w.ospeedr8().bits(0b11));

    gpio.pupdr.modify(|_r, w| unsafe { w.pupdr8().bits(0b01) }); //setting the pull up resistors for pinis PB8 and PB9
    gpio.pupdr.modify(|_r, w| unsafe { w.pupdr9().bits(0b01) });

    gpio.afrh.modify(|_r, w| w.afrh8().bits(0b0100)); //setting the alternate mode to i2c for pins PB8 and PB9
    gpio.afrh.modify(|_r, w| w.afrh9().bits(0b0100));

    i2c.cr1.write(|w| w.swrst().set_bit()); // put i2c in the reset state
    i2c.cr1.modify(|_r, w| w.swrst().clear_bit()); //take i2c out of reset state

    // Program the peripheral input clock in I2c_cr2 register in order to generate correct timings
    i2c.cr2.write(|w| unsafe { w.freq().bits(0b101101) }); //setting periphercal input clock fequency to 45mhz (current max value of APB)
    i2c.cr2.modify(|_r, w| w.itevten().enabled());

    //configure the clock control registers
    i2c.ccr.write(|w| w.f_s().standard());
    i2c.ccr.modify(|_r, w| w.duty().duty2_1());
    i2c.ccr.modify(|_r, w| unsafe { w.ccr().bits(0b11100001) }); //setting crr to 225 (calculated see manual)
    i2c.trise.modify(|_r, w| w.trise().bits(0b101110)); // configure the rise time register (calculated see manual)
    i2c.cr1.modify(|_r, w| w.pe().set_bit()); // Enable the peripheral in i2c_cr1 register

    lcd_init(&mut i2c, &mut timer);
    lcd_put_cur(&mut i2c, 0, &mut col);
    lcd_write_str(&mut i2c, "hello");

    loop {

        // your code goes here
    }
}
