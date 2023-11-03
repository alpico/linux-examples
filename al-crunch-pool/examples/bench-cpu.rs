//! Number crunching benchmark. Checking AVX2 vector add operations.
use al_crunch_pool::*;
use core::arch::asm;
use std::sync::{atomic, Arc};

#[derive(Default)]
struct WorkerState {
    counter: usize,
    value: u64,
}

fn main() {
    /// The runtime for this benchmark.
    const MILLIS: u64 = 10000;
    const INITIAL: u64 = 0x0101010101010101;
    let param = std::env::args().nth(1).unwrap_or("".to_string());
    let info = match param.as_str() {
        "b" | "" => vpaddb::INFO,
        "w" => vpaddw::INFO,
        "d" => vpaddd::INFO,
        "q" => vpaddq::INFO,
        "ps" => vaddps::INFO,
        "pd" => vaddpd::INFO,
        "x" => vpxor::INFO,
        _ => {
            panic!("Invalid mode '{param}' use [b,w,d,q,ps,pd]");
        }
    };
    println!("{} for {MILLIS} ms", info.0);

    let options = Options::default();
    let pool = Pool::new(
        options.clone(),
        (),
        |_| Default::default(),
        |x: WorkerState| x,
    );
    let sender = pool.sender();
    let running = Arc::new(atomic::AtomicBool::new(true));

    // queue one job for any thread
    for _ in 0..options.get_threads() {
        let r = running.clone();
        sender.send(&mut Default::default(), move |state| {
            state.value = unsafe { info.2(&mut state.counter, r, INITIAL) };
        });
    }

    // abort the benchmark after some time.
    let duration = std::time::Duration::from_millis(MILLIS);
    std::thread::sleep(duration);
    running.store(false, std::sync::atomic::Ordering::Release);

    // check the return values
    let mut sum = 0;
    for v in pool.join() {
        // test for the given values
        let counter = v.counter / 32;
        match info.1 {
            1 => assert_eq!(v.value, INITIAL * (counter & 0xff) as u64),
            2 => assert_eq!(
                v.value,
                ((0x0101 * counter) & 0xffff) as u64 * 0x0001_0001_0001_0001
            ),
            4 => assert_eq!(v.value, INITIAL * counter as u64),
            8 => assert_eq!(v.value, INITIAL * counter as u64),
            _ => {}
        }
        sum += v.counter;
    }

    println!(
        "ops {sum} - {} GigaOPS",
        sum / (1_000_000 * info.1.unsigned_abs()) / MILLIS as usize
    );
}

type Func = unsafe fn(*mut usize, Arc<atomic::AtomicBool>, u64) -> u64;

/// A macro to define the assember functions.
macro_rules! benchmark {
    ($y:ident, $size:literal) => {
        mod $y {
            use super::*;
            pub const INFO: (&str, isize, Func) = (stringify!($y), $size, func);

        #[target_feature(enable = "avx")]
        unsafe fn func(counter: *mut usize, running: Arc<atomic::AtomicBool>, initial: u64) -> u64 {
            let mut buf = [0u64; 4*9];
            buf[32] = initial;
            buf[33] = initial;
            buf[34] = initial;
            buf[35] = initial;
            asm!(
                "vmovdqu ymm0, ymmword ptr 0*32[{0}]",
                "vmovdqu ymm1, ymmword ptr 1*32[{0}]",
                "vmovdqu ymm2, ymmword ptr 2*32[{0}]",
                "vmovdqu ymm3, ymmword ptr 3*32[{0}]",
                "vmovdqu ymm4, ymmword ptr 4*32[{0}]",
                "vmovdqu ymm5, ymmword ptr 5*32[{0}]",
                "vmovdqu ymm6, ymmword ptr 6*32[{0}]",
                "vmovdqu ymm7, ymmword ptr 7*32[{0}]",
                "vmovdqu ymm8, ymmword ptr 8*32[{0}]",
                 "2:",
                concat!(stringify!($y)," ymm0, ymm8, ymm0"),
                concat!(stringify!($y)," ymm1, ymm8, ymm1"),
                concat!(stringify!($y)," ymm2, ymm8, ymm2"),
                concat!(stringify!($y)," ymm3, ymm8, ymm3"),
                concat!(stringify!($y)," ymm4, ymm8, ymm4"),
                concat!(stringify!($y)," ymm5, ymm8, ymm5"),
                concat!(stringify!($y)," ymm6, ymm8, ymm6"),
                concat!(stringify!($y)," ymm7, ymm8, ymm7"),
                "add {2:r}, 8*32",
                "cmp dword ptr [{1}], 0",
                "jne 2b",
                concat!(stringify!($y)," ymm6, ymm6, ymm7"),
                concat!(stringify!($y)," ymm4, ymm4, ymm5"),
                concat!(stringify!($y)," ymm2, ymm2, ymm3"),
                concat!(stringify!($y)," ymm0, ymm0, ymm1"),
                concat!(stringify!($y)," ymm4, ymm4, ymm6"),
                concat!(stringify!($y)," ymm0, ymm0, ymm2"),
                concat!(stringify!($y)," ymm0, ymm0, ymm4"),
                "vmovdqu ymmword ptr [{0}], ymm0",
                in(reg) buf.as_ptr(),
                in(reg) running.as_ptr(),
                inout(reg) 0usize => *counter,
            );
            buf[0]
        }
        }
    }
}

benchmark! {vaddpd, -8} // 4x f64 and 250 gigaflops on 8 cores
benchmark! {vaddps, -4} // 8x f32 and 500 gigaflops on 8 cores
benchmark! {vpaddq, 8} // 8x i64 and 500 gigamips on 8 cores
benchmark! {vpaddd, 4} // 8x i32 and 900 gigamips on 8 cores
benchmark! {vpaddw, 2} // 16x i16 and 1800 gigamips on 8 cores
benchmark! {vpaddb, 1} // 32x i8 and 3600 gigamips on 8 cores
benchmark! {vpxor, -1} // 32x i8 and 3600 gigamips on 8 cores
