use std::fs::File;
use std::io::{Seek, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::Command;

use anyhow::{Context, Result};

const BPF_LD_W_ABS: u16 = 0x20;
const BPF_JMP_JEQ_K: u16 = 0x15;
const BPF_JMP_JGE_K: u16 = 0x35;
const BPF_RET_K: u16 = 0x06;
const SECCOMP_DATA_ARCH_OFFSET: u32 = 4;
const SECCOMP_DATA_ARG0_OFFSET: u32 = 16;
const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;
const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
const X32_SYSCALL_BIT: u32 = 0x4000_0000;

#[cfg(target_arch = "x86_64")]
const AUDIT_ARCH: u32 = 0xc000_003e;
#[cfg(target_arch = "aarch64")]
const AUDIT_ARCH: u32 = 0xc000_00b7;

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!("the RUN socket filter needs an audit architecture constant for this target");

fn statement(code: u16, value: u32) -> libc::sock_filter {
    libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k: value,
    }
}

fn jump(value: u32, jump_true: u8, jump_false: u8) -> libc::sock_filter {
    libc::sock_filter {
        code: BPF_JMP_JEQ_K,
        jt: jump_true,
        jf: jump_false,
        k: value,
    }
}

fn socket_filter() -> Vec<libc::sock_filter> {
    vec![
        statement(BPF_LD_W_ABS, SECCOMP_DATA_ARCH_OFFSET),
        jump(AUDIT_ARCH, 1, 0),
        statement(BPF_RET_K, SECCOMP_RET_KILL_PROCESS),
        statement(BPF_LD_W_ABS, 0),
        libc::sock_filter {
            code: BPF_JMP_JGE_K,
            jt: 9,
            jf: 0,
            k: X32_SYSCALL_BIT,
        },
        jump(libc::SYS_socket as u32, 3, 0),
        jump(libc::SYS_socketpair as u32, 2, 0),
        jump(libc::SYS_io_uring_setup as u32, 6, 0),
        statement(BPF_RET_K, SECCOMP_RET_ALLOW),
        statement(BPF_LD_W_ABS, SECCOMP_DATA_ARG0_OFFSET),
        jump(libc::AF_INET as u32, 3, 0),
        jump(libc::AF_INET6 as u32, 2, 0),
        jump(libc::AF_PACKET as u32, 1, 0),
        statement(BPF_RET_K, SECCOMP_RET_ALLOW),
        statement(BPF_RET_K, SECCOMP_RET_ERRNO | libc::EPERM as u32),
    ]
}

fn filter_bytes() -> Vec<u8> {
    let filter = socket_filter();
    let mut bytes = Vec::with_capacity(filter.len() * 8);
    for instruction in filter {
        bytes.extend_from_slice(&instruction.code.to_ne_bytes());
        bytes.push(instruction.jt);
        bytes.push(instruction.jf);
        bytes.extend_from_slice(&instruction.k.to_ne_bytes());
    }
    bytes
}

pub(crate) fn attach_socket_filter(command: &mut Command) -> Result<File> {
    let mut file = tempfile::tempfile().context("creating RUN seccomp filter file")?;
    file.write_all(&filter_bytes())
        .context("writing RUN seccomp filter")?;
    file.rewind().context("rewinding RUN seccomp filter")?;
    let fd = file.as_raw_fd();

    command.arg("--seccomp").arg(fd.to_string());
    unsafe {
        command.pre_exec(move || {
            let flags = libc::fcntl(fd, libc::F_GETFD);
            if flags == -1 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evaluate(syscall: i64, family: i32) -> u32 {
        let data = [syscall as u32, AUDIT_ARCH, 0, 0, family as u32];
        let filter = socket_filter();
        let mut pc = 0;
        let mut accumulator = 0;
        loop {
            let instruction = filter[pc];
            match instruction.code {
                BPF_LD_W_ABS => {
                    accumulator = data[(instruction.k / 4) as usize];
                    pc += 1;
                }
                BPF_JMP_JEQ_K => {
                    pc += 1 + if accumulator == instruction.k {
                        instruction.jt as usize
                    } else {
                        instruction.jf as usize
                    };
                }
                BPF_JMP_JGE_K => {
                    pc += 1 + if accumulator >= instruction.k {
                        instruction.jt as usize
                    } else {
                        instruction.jf as usize
                    };
                }
                BPF_RET_K => return instruction.k,
                code => panic!("unexpected cBPF opcode {code:#x}"),
            }
        }
    }

    #[test]
    fn filter_bytes_deny_internet_socket_families_and_allow_unix() {
        assert_eq!(
            evaluate(libc::SYS_socket, libc::AF_INET),
            SECCOMP_RET_ERRNO | libc::EPERM as u32
        );
        assert_eq!(
            evaluate(libc::SYS_socket, libc::AF_INET6),
            SECCOMP_RET_ERRNO | libc::EPERM as u32
        );
        assert_eq!(
            evaluate(libc::SYS_socketpair, libc::AF_PACKET),
            SECCOMP_RET_ERRNO | libc::EPERM as u32
        );
        assert_eq!(evaluate(libc::SYS_socket, libc::AF_UNIX), SECCOMP_RET_ALLOW);
        assert_eq!(
            evaluate(libc::SYS_socketpair, libc::AF_UNIX),
            SECCOMP_RET_ALLOW
        );
        assert_eq!(
            evaluate(libc::SYS_io_uring_setup, 0),
            SECCOMP_RET_ERRNO | libc::EPERM as u32
        );
        assert_eq!(
            evaluate(libc::SYS_socket | X32_SYSCALL_BIT as i64, libc::AF_INET),
            SECCOMP_RET_ERRNO | libc::EPERM as u32
        );
        assert_eq!(filter_bytes().len(), 15 * 8);
    }

    #[test]
    fn kernel_enforces_filter_for_inet_and_permits_unix() {
        let filter = socket_filter();
        let child = unsafe { libc::fork() };
        assert_ne!(child, -1, "fork failed");
        if child == 0 {
            let program = libc::sock_fprog {
                len: filter.len() as u16,
                filter: filter.as_ptr().cast_mut(),
            };
            let no_new_privileges = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
            if no_new_privileges != 0 {
                unsafe { libc::_exit(10) };
            }
            let installed = unsafe {
                libc::syscall(
                    libc::SYS_seccomp,
                    libc::SECCOMP_SET_MODE_FILTER,
                    0,
                    &program,
                )
            };
            if installed != 0 {
                unsafe { libc::_exit(11) };
            }
            let inet = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
            if inet != -1 || unsafe { *libc::__errno_location() } != libc::EPERM {
                unsafe { libc::_exit(12) };
            }
            let unix = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
            if unix == -1 {
                unsafe { libc::_exit(13) };
            }
            unsafe {
                libc::close(unix);
                libc::_exit(0);
            }
        }

        let mut status = 0;
        assert_eq!(unsafe { libc::waitpid(child, &mut status, 0) }, child);
        assert!(
            libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0,
            "filtered child exited with wait status {status}"
        );
    }
}
