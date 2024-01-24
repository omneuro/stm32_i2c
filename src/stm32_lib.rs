pub mod rcc {
    use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                         // use panic_abort as _; // requires nightly
                         // use panic_itm as _; // logs messages over ITM; requires ITM support
                         // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

    use stm32f4::stm32f446;

    pub fn initialize_clock(
        rcc: &mut stm32f446::RCC,
        pwr: &mut stm32f446::PWR,
        flash: &mut stm32f446::FLASH,
        //timer: &mut stm32f446::TIM11,
    ) {
        rcc.cr.write(|w| w.hseon().set_bit()); // Turning on the high speed external oscillator
        while rcc.cr.read().hserdy().bit() { // waiting for the high speed external oscillator ready bit to be set
             // Do noting while the hserdy bit is not set (i.e 0)
        }

        // Set the POWER ENABLE CLOCK and VOLTAGE REGULATOR
        rcc.apb1enr.write(|w| w.pwren().set_bit()); //setting power enable clock bit
        pwr.cr.write(|w| unsafe { w.vos().bits(0b10) });

        //Configure the Flas and Prefetch and the latency related settings
        flash.acr.write(|w| w.icen().set_bit());
        flash.acr.modify(|_r, w| w.dcen().set_bit());
        flash.acr.modify(|_r, w| w.prften().set_bit());
        flash.acr.modify(|_r, w| w.latency().bits(0b0101));

        // Configure AHB, APB1 and APB2 using the RCC_CFGR (division factor prescalers)
        rcc.cfgr.write(|w| unsafe { w.ppre1().bits(0b100) }); //writing 4 to APB1
        rcc.cfgr.modify(|_r, w| unsafe { w.ppre2().bits(0b010) }); //writing 2 to APB2

        //Configure the Main PLL
        rcc.pllcfgr.write(|w| unsafe { w.pllm().bits(0b000100) }); // setting PLLM to 4
        rcc.pllcfgr
            .modify(|_r, w| unsafe { w.plln().bits(0b010110100) }); // setting PLLN to 180
        rcc.pllcfgr.modify(|_r, w| w.pllp().bits(0b01)); //setting PLLP to 4. i.e option 01

        //Enable the PLL and wait for it to be ready
        rcc.cr.modify(|_r, w| w.pllon().set_bit());
        while rcc.cr.read().pllrdy().bit() {
            //Do noting while pllrdy bit is not set
        }

        //Select the clock source and wait for it to be on
        rcc.cfgr.modify(|_r, w| unsafe { w.sw().bits(0b10) });
        while rcc.cfgr.read().sws().bits() != 0b10 {
            //do noting while clock source is not set
        }
    }

    // delay function implemented using timer 11
    pub fn delay_us(us: u16, timer: &mut stm32f446::TIM11) {
        timer.cnt.write(|w| unsafe { w.cnt().bits(0x0000) });
        while timer.cnt.read().cnt().bits() < us {}
    }

    pub fn delay_ms(ms: u16, timer: &mut stm32f446::TIM11) {
        for _ in 0..ms {
            delay_us(1000, timer);
        }
    }
}
