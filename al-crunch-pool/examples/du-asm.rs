//! A disk usage implementation implemented in assembly.

#![feature(rustc_private)]
#![feature(naked_functions)]
#![feature(asm_const)]

use al_crunch_pool::Pool;
extern crate libc;

#[repr(C)]
#[derive(Default)]
struct Data {
    count: u64,
    blocks: u64,
}

/// The wrapper function for the inner one.
fn visit(fd: i32, name: *const i8) -> Data {
    let mut res = Data {
        count: 0,
        blocks: 0,
    };
    unsafe {
        core::arch::asm!("call visit_inner",
                         in("rdi") fd,
                         inout("rsi") name => _,
                         inout("r14") 0u64 => res.count,
                         inout("r15") 0u64 => res.blocks,
                         out("rax") _,
                         out("rdx") _,
                         out("rcx") _,
                         in("r10") libc::AT_SYMLINK_NOFOLLOW,
                         out("r11") _,
                         out("r12") _,
                         out("r13") _,
        )
    }
    res
}

/// Internal function no to be called directly.
///
/// rdi - the parent file descriptor
/// rsi - the name inside the parent and used as buffer pointer
/// rdx - syscall parameters
/// r10 - flags for fstatat
/// rcx - scratch in syscall
/// r11 - scratch in syscall
/// r12 - the offset into the buffer
/// r13 - the bytes returned by SYS_getdents
/// r14 - the file counter
/// r15 - the blocks counter
#[naked]
#[no_mangle]
unsafe extern "C" fn visit_inner() {
    core::arch::asm!(
        "sub rsp, {STAT_SIZE} + {BUF_SIZE}",

        // open the directory
        "mov rax, {SYS_openat}",
        // rdi is live
        // rsi has our name
        "mov rdx, {FLAGS_OPEN}",
        "syscall",
        "cmp rax, 0",
        "jge 1f",
        // error handling if the open failed
        "ud2",
        "1:",
        
        // keep the fd inside rdi
        "mov rdi, rax",

        // the outer loop - get the dents
        "2:",
        "mov rax, {SYS_getdents64}",
        "mov rsi, rsp",
        "mov rdx, {BUF_SIZE}",
        "syscall",
        // keep the count in r13
        "mov r13, rax",
        "xor r12, r12", // r12 contains the offset into the buffer
        // rax has the error code
        "cmp rax, 0",
        // some entries - go to inner loop
        "jg 20f",
        // eof?
        "je 3f",
        // error handling needed if getdents failed
        "ud2",

        // we are done
        "3:",
       
        // close the file
        "mov rax, {SYS_close}",
        "syscall",
        "cmp rax, 0",
        "je 4f",
        // error handling if the close failed - this should never happen
        "ud2",

        // cleanup the stack
        "4:",
        "add rsp, {STAT_SIZE} + {BUF_SIZE}",
        "ret",


        // the inner loop
        "20:",

        // put the name in RSI - either we stat or we will open it
        "lea rsi, 19[r12+rsp]",

        // compare to the single dot as name - just stat
        "cmp word ptr [rsi], 0x002e",
        "je 40f",

        // skip the parent directory 
        "cmp dword ptr -1[rsi], 0x002e2e04",
        "je 30f",

        // is it a directory? If not take the stat case
        "cmp byte ptr -1[rsi], 4",
        "jne 40f",


        // this is a directory to be visited recursively
        "push rdi",
        "push r12",
        "push r13",
        "call visit_inner",
        "pop r13",
        "pop r12",
        "pop rdi",

        // done with this entry
        "30:",
        // check the offset for indicating the end of the dentries
        "mov rax, {END_OFFSET}",
        "cmp qword ptr 8[r12+rsp], rax",
        "je 3b",
        
        // advance the rec-len
        "add r12w, word ptr 16[r12+ rsp]",
        "cmp r13, r12",
        // next entry - inner loop
        "jg 20b",
        // refill the buffer
        "je 2b",
        // error handling needed if the buffer is overflown
        "ud2",

        
        "40:",
        // normal files or this directory - count it
        "inc r14",

        // stat the file to get the blocks occupied
        // rdi is the directory FD
        "mov rax, {SYS_newfstatat}",
        // take space above the dentry buffer
        "lea rdx, {BUF_SIZE}[rsp]",
        "syscall",
        "cmp rax, 0",
        "je 41f",
        // error handling if the stat failed
        "ud2",

        "41:",
        // add the st_blocks field from the STAT buffer
        "add r15, 64+{BUF_SIZE}[rsp]",
        "jmp 30b",
       


        STAT_SIZE = const core::mem::size_of::<libc::stat>(),
        BUF_SIZE = const 16384,
        FLAGS_OPEN = const libc::O_DIRECTORY | libc::O_RDONLY,
        END_OFFSET = const i64::MAX,
        SYS_getdents64=const libc::SYS_getdents64,
        SYS_openat=const libc::SYS_openat,
        SYS_newfstatat=const libc::SYS_newfstatat,
        SYS_close=const libc::SYS_close,
        options(noreturn))
}

#[derive(Default)]
pub struct WorkerState {
    nr: usize,
    data: Data,
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let pool = Pool::default();
    for (nr, path) in paths.iter().enumerate() {
        let cpath = std::ffi::CString::new(path.clone()).unwrap();
        pool.sender()
            .send(&mut Default::default(), move |state: &mut WorkerState| {
                state.nr = nr + 1;
                state.data = visit(libc::AT_FDCWD, cpath.as_ptr());
            });
    }

    for v in pool.join() {
        if v.nr == 0 {
            continue;
        };
        println!(
            "{} {} {}",
            paths[v.nr - 1],
            v.data.count,
            v.data.blocks << 9
        );
    }
}
