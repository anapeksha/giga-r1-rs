#![no_std]

use aligned::{A4, Aligned};

/// Length of Arduino's CYW4343W firmware image.
pub const FIRMWARE_LEN: usize = 421_098;

/// Length of the matching Country Locale Matrix.
pub const CLM_LEN: usize = 7_222;

/// Arduino's CYW4343W firmware image, aligned for SDIO transfers.
pub static FIRMWARE: Aligned<A4, [u8; FIRMWARE_LEN]> = Aligned(*include_bytes!("../4343WA1.bin"));

/// Country Locale Matrix matching [`FIRMWARE`].
pub static CLM: &[u8; CLM_LEN] = include_bytes!("../4343WA1.clm_blob");

/// CRC-32/ISO-HDLC of [`FIRMWARE`].
pub const FIRMWARE_CRC32: u32 = 0xeafe_5f02;

/// Arduino's GIGA NVRAM configuration for the onboard Murata Type 1DX.
pub static NVRAM: Aligned<A4, [u8; 562]> = Aligned(
    *b"manfid=0x2d0\0\
prodid=0x0726\0\
vendid=0x14e4\0\
devid=0x43e2\0\
boardtype=0x0726\0\
boardrev=0x1202\0\
boardnum=22\0\
macaddr=02:00:00:00:47:01\0\
sromrev=11\0\
boardflags=0x00404201\0\
boardflags3=0x04000000\0\
xtalfreq=37400\0\
nocrc=1\0\
ag0=0\0\
aa2g=1\0\
ccode=ALL\0\
extpagain2g=0\0\
pa2ga0=-145,6667,-751\0\
AvVmid_c0=0x0,0xc8\0\
cckpwroffset0=2\0\
maxp2ga0=74\0\
cckbw202gpo=0\0\
legofdmbw202gpo=0x88888888\0\
mcsbw202gpo=0xaaaaaaaa\0\
propbw202gpo=0xdd\0\
ofdmdigfilttype=18\0\
ofdmdigfilttypebe=18\0\
papdmode=1\0\
papdvalidtest=1\0\
pacalidx2g=48\0\
papdepsoffset=-22\0\
papdendidx=58\0\
il0macaddr=02:00:00:00:47:01\0\
wl0id=0x431b\0\
muxenab=0x10\0\0\0",
);
