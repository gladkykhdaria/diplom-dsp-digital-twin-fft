
use embassy_rp::adc::InterruptHandler as AdcInterruptHandler;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c::InterruptHandler as I2cInterruptHandler;
use embassy_rp::peripherals::I2C0;

bind_interrupts!(
    pub(super) struct Irqs {
        ADC_IRQ_FIFO => AdcInterruptHandler;
        I2C0_IRQ => I2cInterruptHandler<I2C0>;
    }
);
