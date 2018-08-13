// This file heavilly based on https://github.com/redox-os/tfs/blob/master/lz4/src/decompress.rs
// Therefore, it's licenced under the MIT license, Copyright (c) 2016 Ticki, 2018 Kitlith

// TODO: allow for streaming decompression

use byteorder::{BigEndian, ByteOrder};

quick_error! {
    /// An error representing invalid compressed data.
    #[derive(Debug)]
    pub enum Error {
        /// Expected another byte, but none found.
        ExpectedAnotherByte {
            description("Expected another byte, found none.")
        }
        /// Deduplication offset out of bounds (not in buffer).
        OffsetOutOfBounds {
            description("The offset to copy is not contained in the decompressed buffer.")
        }

        InvalidMagic {
            description("Invalid compression magic.")
        }
    }
}

use self::Error::*;

enum Command {
    Normal {
        preceeding: usize,
        len: usize,
        offset: usize,
    },
    Stop {
        preceeding: usize,
    },
}

/// A refpack/qfs/ea_jdlz decoder
///
/// This attempts to decompress sections in DBPF files. Heavily based on Ticki's lz4 decoder.
struct Decoder<'a> {
    /// The compressed input.
    input: &'a [u8],
    /// The decompressed output.
    output: &'a mut Vec<u8>,
    // maybe the current command?
}

impl<'a> Decoder<'a> {
    /// Internal (partial) function for `take`.
    #[inline]
    fn take_imp(input: &mut &'a [u8], n: usize) -> Result<&'a [u8], Error> {
        // Check if we have enough bytes left.
        if input.len() < n {
            // No extra bytes. This is clearly not expected, so we return an error.
            Err(ExpectedAnotherByte)
        } else {
            // Take the first n bytes.
            let res = Ok(&input[..n]);
            // Shift the stream to left, so that it is no longer the first byte.
            *input = &input[n..];

            // Return the former first byte.
            res
        }
    }

    /// Pop n bytes from the start of the input stream.
    fn take(&mut self, n: usize) -> Result<&[u8], Error> {
        Self::take_imp(&mut self.input, n)
    }

    /// Write a buffer to the output stream.
    ///
    /// The reason this doesn't take `&mut self` is that we need partial borrowing due to the rules
    /// of the borrow checker. For this reason, we instead take some number of segregated
    /// references so we can read and write them independently.
    fn output(output: &mut Vec<u8>, buf: &[u8]) {
        // We use simple memcpy to extend the vector.
        output.extend_from_slice(buf);
    }

    /// Write an already decompressed match to the output stream.
    ///
    /// This is used for the essential part of the algorithm: deduplication. We start at some
    /// position `start` and then keep pushing the following element until we've added
    /// `match_length` elements.
    fn duplicate(&mut self, start: usize, match_length: usize) {
        // We cannot simply use memcpy or `extend_from_slice`, because these do not allow
        // self-referential copies: http://ticki.github.io/img/lz4_runs_encoding_diagram.svg
        for i in start..start + match_length {
            let b = self.output[i];
            self.output.push(b);
        }
    }

    /// Read a command from the stream
    fn read_command(&mut self) -> Result<Command, Error> {
        // Unary encoding, to figure out the command width
        if self.input[0] & (1 << 7) == 0 {
            // 2 bytes: 0OOLLLPP OOOOOOOO
            let cmd = self.take(2)?;
            Ok(Command::Normal {
                preceeding: cmd[0] as usize & 3,
                len: ((cmd[0] as usize & 0b00011100) >> 2) + 3,
                offset: ((cmd[0] as usize & 0b01100000) << 3) + cmd[1] as usize,
            })
        } else if self.input[0] & (1 << 6) == 0 {
            // 3 bytes: 10LLLLLL PPOOOOOO OOOOOOOO
            let cmd = self.take(3)?;
            Ok(Command::Normal {
                preceeding: (cmd[1] as usize & 0b11000000) >> 6,
                len: (cmd[0] as usize & 0b00111111) + 4,
                offset: ((cmd[1] as usize & 0b00111111) << 8) + cmd[2] as usize,
            })
        } else if self.input[0] & (1 << 5) == 0 {
            // 4 bytes: 110OLLPP OOOOOOOO OOOOOOOO LLLLLLLL
            let cmd = self.take(4)?;
            Ok(Command::Normal {
                preceeding: cmd[0] as usize & 0b00000011,
                len: ((cmd[0] as usize & 0b00001100) << 6) + cmd[3] as usize + 5,
                offset: ((cmd[0] as usize & 0b00010000) << 12)
                    + ((cmd[1] as usize) << 8)
                    + cmd[2] as usize,
            })
        } else {
            // 1 byte:  111PPPPP
            let cmd = self.take(1)?[0];
            if (cmd & 0b00011100) == 0b00011100 {
                Ok(Command::Stop {
                    preceeding: cmd as usize & 0b00000011,
                })
            } else {
                Ok(Command::Normal {
                    preceeding: ((cmd as usize & 0b00011111) + 1) << 2,
                    len: 0,
                    offset: 0,
                })
            }
        }
    }

    /// Complete the decompression by reading all the blocks.
    fn complete(&mut self) -> Result<(), Error> {
        let flags = self.take(1)?[0];
        let magic = self.take(1)?[0];
        if magic != 0xFB {
            return Err(InvalidMagic);
        }

        let large = (flags & (1 << 7)) != 0;
        // let unknown = (flags & (1 << 6)) != 0;
        let compressed_size_present = (flags & 1) != 0;

        let _compressed_size = if compressed_size_present {
            if large {
                Some(BigEndian::read_u32(self.take(4)?))
            } else {
                Some(BigEndian::read_u24(self.take(3)?))
            }
        } else {
            None
        };

        let decompressed_size = if large {
            BigEndian::read_u32(self.take(4)?)
        } else {
            BigEndian::read_u24(self.take(3)?)
        } as usize;

        self.output.reserve_exact(decompressed_size);

        while !self.input.is_empty() {
            match self.read_command()? {
                Command::Normal {
                    preceeding,
                    len,
                    offset,
                } => {
                    // I would much rather use this version, but it doesn't borrow check.
                    // self.output.extend_from_slice(self.take(preceeding)?);

                    Self::output(
                        &mut self.output,
                        Self::take_imp(&mut self.input, preceeding)?,
                    );
                    //if len == 0 { continue; }

                    // Calculate the start of this duplicate segment. We use wrapping subtraction
                    // to avoid overflow checks, which we catch manually to avoid panics.
                    let start = self.output.len().wrapping_sub(offset as usize + 1);
                    if start < self.output.len() {
                        // Write the duplicate section to the output buffer.
                        self.duplicate(start, len);
                    } else {
                        return Err(OffsetOutOfBounds);
                    }
                }
                Command::Stop { preceeding } => {
                    Self::output(
                        &mut self.output,
                        Self::take_imp(&mut self.input, preceeding)?,
                    );
                    break;
                }
            };
        }
        Ok(())
    }
}

/// Decompress all bytes of `input` into `output`.
pub fn decompress_into(input: &[u8], output: &mut Vec<u8>) -> Result<(), Error> {
    // Decode into our vector.
    Decoder {
        input: input,
        output: output,
    }.complete()?;

    Ok(())
}

/// Decompress all bytes of `input`.
pub fn decompress(input: &[u8]) -> Result<Vec<u8>, Error> {
    // Allocate a vector to contain the decompressed stream.
    let mut vec = Vec::with_capacity(0); // We grow it later.

    decompress_into(input, &mut vec)?;

    Ok(vec)
}
