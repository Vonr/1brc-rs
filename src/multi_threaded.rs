use std::{
    collections::HashMap,
    fs::File,
    hash::BuildHasherDefault,
    io::{self, BufWriter, Write},
    num::NonZeroI32,
};

mod data;
mod itoa;

use data::Data;

const CORES: usize = usize::from_le_bytes(*include_bytes!(concat!(env!("OUT_DIR"), "/core_count")));

fn main() {
    unsafe {
        let mmap = memmap2::Mmap::map(&File::open("measurements.txt").unwrap()).unwrap();
        let mmap = &mmap[..];

        let len = mmap.len();
        let chunk_size = len / CORES;

        let mut v = std::thread::scope(|s| {
            std::array::from_fn::<_, CORES, _>(|i| {
                s.spawn(move || {
                    let base = i * chunk_size;
                    let chunk_start = if i == 0 {
                        0
                    } else {
                        base + mmap[base..]
                            .iter()
                            .position(|&b| b == b'\n')
                            .unwrap_unchecked()
                            + 1
                    };
                    let chunk_end = mmap[base + chunk_size..]
                        .iter()
                        .position(|&b| b == b'\n')
                        .map(|n| base + chunk_size + n - 1)
                        .unwrap_unchecked();

                    if chunk_start > chunk_end {
                        return Default::default();
                    }

                    let chunk = mmap.get_unchecked(chunk_start..=chunk_end);

                    let mut map = HashMap::<&str, Data, _>::with_capacity_and_hasher(
                        10_000,
                        BuildHasherDefault::<rustc_hash::FxHasher>::default(),
                    );

                    for line in chunk.split(|&b| b == b'\n') {
                        let at = line.iter().position(|&b| b == b';').unwrap_unchecked();

                        let station = line.get_unchecked(..at);
                        let temp_bytes = line.get_unchecked(at + 1..);

                        let mut int = &temp_bytes[..temp_bytes.len() - 2];
                        let dec = temp_bytes[temp_bytes.len() - 1];

                        let temp = {
                            let neg = *int.get_unchecked(0) == b'-';
                            if neg {
                                int = int.get_unchecked(1..);
                            };

                            let temp = match int.len() {
                                1 => int.get_unchecked(0).wrapping_sub(b'0') as i16,
                                2 => {
                                    int.get_unchecked(0).wrapping_sub(b'0') as i16 * 10
                                        + int.get_unchecked(1).wrapping_sub(b'0') as i16
                                }
                                _ => std::hint::unreachable_unchecked(),
                            } * 10
                                + dec.wrapping_sub(b'0') as i16;

                            if neg {
                                -temp
                            } else {
                                temp
                            }
                        };

                        map.entry(std::str::from_utf8_unchecked(station))
                            .and_modify(|e| {
                                *e = Data {
                                    total: e.total + temp as i32,
                                    count: NonZeroI32::new_unchecked(e.count.get() + 1),
                                    min: e.min.min(temp),
                                    max: e.max.max(temp),
                                };
                            })
                            .or_insert(Data {
                                total: temp as i32,
                                count: NonZeroI32::new_unchecked(1),
                                min: temp,
                                max: temp,
                            });
                    }

                    map
                })
            })
            .map(|h| h.join().unwrap_unchecked())
        })
        .into_iter()
        .reduce(|mut base, next| {
            for (k, v) in next.into_iter() {
                base.entry(k)
                    .and_modify(|e| {
                        *e = Data {
                            total: e.total + v.total,
                            count: NonZeroI32::new_unchecked(e.count.get() + v.count.get()),
                            min: e.min.min(v.min),
                            max: e.max.max(v.max),
                        };
                    })
                    .or_insert(v);
            }

            base
        })
        .unwrap_unchecked()
        .into_iter()
        .collect::<Box<_>>();

        glidesort::sort(&mut v);

        let stdout = io::stdout().lock();
        let mut writer = BufWriter::new(stdout);

        for (
            station,
            Data {
                total,
                count,
                min,
                max,
            },
        ) in v.into_iter()
        {
            writer.write_all(station.as_bytes()).unwrap_unchecked();
            writer
                .write_all(itoa::format(*min).as_bytes())
                .unwrap_unchecked();
            writer
                .write_all(
                    itoa::format((*total as f64 / count.get() as f64).round() as i16).as_bytes(),
                )
                .unwrap_unchecked();
            writer
                .write_all(itoa::format(*max).as_bytes())
                .unwrap_unchecked();
            writer.write_all(b"\n").unwrap_unchecked();
        }

        std::mem::forget(v);
    }
}
