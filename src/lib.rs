/* read and write an MMIO-based NS 16550-compatible UART
 * 
 * Datasheet: https://www.nxp.com/docs/en/data-sheet/SC16C550B.pdf
 * 
 * (c) Chris Williams, 2020.
 *
 * See README and LICENSE for usage and copying.
 */

/* we're on our own here */
#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]

use core::ptr::{write_volatile, read_volatile};

const REG_TOTAL_SIZE: usize = 8;        /* 8 byte registers */

/* registers 0-7 in the 16550 */
const REG_DATA: usize = 0;              /* byte to transmit or receive */
const REG_DIVISOR_LSB: usize = 0;       /* least sig byte of divisor in DLAB mode */
const REG_DIVISOR_MSB: usize = 1;       /* most sig byte of divisor in DLAB mode */
const REG_IRQ_EN: usize = 1;            /* interrupt enable */
const REG_FIFO_CONTROL: usize = 2;      /* FIFO and IRQ id control */
const REG_LINE_CONTROL: usize = 3;      /* communications control bits */
const REG_MODEM_CONTROL: usize = 4;     /* modem control bits */
const REG_LINE_STATUS: usize = 5;       /* communications status bits */

/* define line control bits */
const LINE_CONTROL_DLAB: u8 = 1 << 7;   /* enable divisor latch access bit (DLAB) */

/* define line status bits */
const LINE_STATUS_DR: u8 = 1 << 0;      /* data ready */
const LINE_STATUS_THRE: u8 = 1 << 5;    /* transmitter holding register empty */

/* to avoid infinite loops, give up checking
   for a byte to arrive or for a byte to be
   transmitted after this many check iterations */
const LOOP_MAX: usize = 1000;

/* possible error conditions supported at this time */
#[derive(Debug)]
pub enum Fault
{
    TxNotEmpty,     /* gave up waiting to transmit */
    DataNotReady    /* gave up waiting to send */
}

#[derive(Debug)]
pub struct UART
{
    base_addr: usize
}

impl UART
{
    /* create and initialize a standard 8-n-1 UART object, or fail with a reason code.
    TODO: Configure this initialization */
    pub fn new(base_addr: usize) -> Result<Self, Fault>
    {
        let uart = UART { base_addr };

        /* disable IRQs from this chip */
        uart.write_reg(REG_IRQ_EN, 0);

        /* enable DLAB, set speed to 38400 bps, disable DLAB,
        and set data 8 bits in length, no parity, one stop bit */
        uart.write_reg(REG_LINE_CONTROL, LINE_CONTROL_DLAB);
        uart.write_reg(REG_DIVISOR_LSB, 3); // 115200 / 3 = 38400 bps
        uart.write_reg(REG_DIVISOR_MSB, 0);
        uart.write_reg(REG_LINE_CONTROL, 0b0011); // len = 8, 1 stop bit, no parity, dlab = 0

        /* enable FIFO, set IRQ watermark to 14 bytes */
        uart.write_reg(REG_FIFO_CONTROL, 0xc7);

        /* enable IRQ line 1, clear RTS and DTR */
        uart.write_reg(REG_MODEM_CONTROL, 0b1011);

        /* enable IRQs */
        uart.write_reg(REG_IRQ_EN, 1);

        Ok(uart)
    }

    /* return size of this controller's MMIO space in bytes */
    pub fn size(&self) -> usize
    {
        REG_TOTAL_SIZE
    }

    /* centralize reading and writing of registers to these unsafe functions */
    fn write_reg(&self, reg: usize, val: u8)
    {
        unsafe { write_volatile((self.base_addr + reg) as *mut u8, val) }
    }

    fn read_reg(&self, reg: usize) -> u8
    {
        unsafe { read_volatile((self.base_addr + reg) as *const u8) }
    }

    pub fn send_byte(&self, to_send: u8) -> Result<(), Fault>
    {
        for _ in 0..LOOP_MAX
        {
            if self.is_transmit_empty() == true
            {
                self.write_reg(REG_DATA, to_send);
                return Ok(());
            }
        }

        Err(Fault::TxNotEmpty)
    }

    pub fn read_byte(&self) -> Result<u8, Fault>
    {
        for _ in 0..LOOP_MAX
        {
            if self.is_data_ready() == true
            {
                return Ok(self.read_reg(REG_DATA));
            }   
        }

        Err(Fault::DataNotReady)
    }

    /* return true if data can be sent */
    fn is_transmit_empty(&self) -> bool
    {
        let val = self.read_reg(REG_LINE_STATUS);
        return val & LINE_STATUS_THRE != 0
    }

    /* return true if data is ready to be read */
    fn is_data_ready(&self) -> bool
    {
        let val = self.read_reg(REG_LINE_STATUS);
        return val & LINE_STATUS_DR != 0
    }
}

#[cfg(test)]
mod tests
{
    #[test]
    fn it_works()
    {
        assert_eq!(2 + 2, 4);
    }
}
