//! 沙箱执行的资源限制

use std::num::NonZeroUsize;

/// 配置进程的资源限制
pub struct ResourceLimitConfig {
    /// 最大内存（字节）
    pub max_memory_bytes: NonZeroUsize,
    /// 最大 CPU 时间（秒）
    pub max_cpu_seconds: u64,
    /// 最大线程数
    pub max_threads: Option<NonZeroUsize>,
    /// 最大打开文件数
    pub max_open_files: Option<NonZeroUsize>,
    /// 最大文件大小（字节）
    pub max_file_size_bytes: Option<NonZeroUsize>,
}

impl Default for ResourceLimitConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: NonZeroUsize::new(256 * 1024 * 1024).unwrap(), // 256MB
            max_cpu_seconds: 10,
            max_threads: NonZeroUsize::new(4),
            max_open_files: NonZeroUsize::new(64),
            max_file_size_bytes: Some(NonZeroUsize::new(100 * 1024 * 1024).unwrap()), // 100MB
        }
    }
}

impl ResourceLimitConfig {
    /// 创建新的配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置内存限制
    pub fn with_memory(mut self, bytes: usize) -> Self {
        if let Some(limit) = NonZeroUsize::new(bytes) {
            self.max_memory_bytes = limit;
        }
        self
    }

    /// 设置 CPU 时间限制
    pub fn with_cpu_time(mut self, seconds: u64) -> Self {
        self.max_cpu_seconds = seconds;
        self
    }

    /// 设置线程数限制
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.max_threads = NonZeroUsize::new(threads);
        self
    }

    /// 设置打开文件数限制
    pub fn with_open_files(mut self, files: usize) -> Self {
        self.max_open_files = NonZeroUsize::new(files);
        self
    }

    /// 设置文件大小限制
    pub fn with_file_size(mut self, bytes: usize) -> Self {
        self.max_file_size_bytes = NonZeroUsize::new(bytes);
        self
    }

    /// 构建 ulimit 参数
    pub fn to_ulimit_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // 内存限制
        args.push(format!("-v{}", self.max_memory_bytes.get()));

        // CPU 时间限制
        args.push(format!("-t{}", self.max_cpu_seconds));

        // 最大线程数（通过 ulimit -u）
        if let Some(threads) = self.max_threads {
            args.push(format!("-u{}", threads.get()));
        }

        // 最大打开文件数
        if let Some(files) = self.max_open_files {
            args.push(format!("-n{}", files.get()));
        }

        // 最大文件大小
        if let Some(size) = self.max_file_size_bytes {
            args.push(format!("-f{}", size.get()));
        }

        args
    }
}

/// seccomp 过滤器的允许系统调用列表
#[derive(Debug, Clone)]
pub struct SyscallAllowList {
    /// 系统调用列表
    pub syscalls: Vec<i32>,
}

impl SyscallAllowList {
    /// 创建新的允许列表
    #[cfg(target_os = "linux")]
    pub fn new() -> Self {
        // 大多数程序所需的基本系统调用
        let syscalls = vec![
            libc::SYS_read,
            libc::SYS_write,
            libc::SYS_open,
            libc::SYS_close,
            libc::SYS_stat,
            libc::SYS_fstat,
            libc::SYS_lstat,
            libc::SYS_mmap,
            libc::SYS_mprotect,
            libc::SYS_munmap,
            libc::SYS_brk,
            libc::SYS_rt_sigaction,
            libc::SYS_rt_sigprocmask,
            libc::SYS_rt_sigreturn,
            libc::SYS_ioctl,
            libc::SYS_access,
            libc::SYS_pipe,
            libc::SYS_select,
            libc::SYS_mremap,
            libc::SYS_msync,
            libc::SYS_mincore,
            libc::SYS_madvise,
            libc::SYS_shmget,
            libc::SYS_shmat,
            libc::SYS_shmctl,
            libc::SYS_dup,
            libc::SYS_dup2,
            libc::SYS_pause,
            libc::SYS_nanosleep,
            libc::SYS_getitimer,
            libc::SYS_alarm,
            libc::SYS_setitimer,
            libc::SYS_getpid,
            libc::SYS_sendfile,
            libc::SYS_socket,
            libc::SYS_connect,
            libc::SYS_accept,
            libc::SYS_sendto,
            libc::SYS_recvfrom,
            libc::SYS_sendmsg,
            libc::SYS_recvmsg,
            libc::SYS_shutdown,
            libc::SYS_bind,
            libc::SYS_listen,
            libc::SYS_getsockname,
            libc::SYS_getpeername,
            libc::SYS_socketpair,
            libc::SYS_setsockopt,
            libc::SYS_getsockopt,
            libc::SYS_clone,
            libc::SYS_fork,
            libc::SYS_vfork,
            libc::SYS_execve,
            libc::SYS_exit,
            libc::SYS_wait4,
            libc::SYS_kill,
            libc::SYS_uname,
            libc::SYS_semget,
            libc::SYS_semop,
            libc::SYS_semctl,
            libc::SYS_shmdt,
            libc::SYS_msgget,
            libc::SYS_msgsnd,
            libc::SYS_msgrcv,
            libc::SYS_msgctl,
            libc::SYS_fcntl,
            libc::SYS_flock,
            libc::SYS_fsync,
            libc::SYS_fdatasync,
            libc::SYS_truncate,
            libc::SYS_ftruncate,
            libc::SYS_getdents,
            libc::SYS_getcwd,
            libc::SYS_chdir,
            libc::SYS_fchdir,
            libc::SYS_rename,
            libc::SYS_mkdir,
            libc::SYS_rmdir,
            libc::SYS_creat,
            libc::SYS_link,
            libc::SYS_unlink,
            libc::SYS_symlink,
            libc::SYS_readlink,
            libc::SYS_chmod,
            libc::SYS_fchmod,
            libc::SYS_chown,
            libc::SYS_fchown,
            libc::SYS_lchown,
            libc::SYS_umask,
            libc::SYS_gettimeofday,
            libc::SYS_getrlimit,
            libc::SYS_getrusage,
            libc::SYS_sysinfo,
            libc::SYS_times,
            libc::SYS_getuid,
            libc::SYS_syslog,
            libc::SYS_getgid,
            libc::SYS_setuid,
            libc::SYS_setgid,
            libc::SYS_geteuid,
            libc::SYS_getegid,
            libc::SYS_setpgid,
            libc::SYS_getpgrp,
            libc::SYS_setsid,
            libc::SYS_setreuid,
            libc::SYS_setregid,
            libc::SYS_getgroups,
            libc::SYS_setgroups,
            libc::SYS_setresuid,
            libc::SYS_getresuid,
            libc::SYS_setresgid,
            libc::SYS_getresgid,
            libc::SYS_getpgid,
            libc::SYS_setfsuid,
            libc::SYS_setfsgid,
            libc::SYS_getsid,
            libc::SYS_capget,
            libc::SYS_capset,
            libc::SYS_rt_sigpending,
            libc::SYS_rt_sigsuspend,
            libc::SYS_sigaltstack,
            libc::SYS_utime,
            libc::SYS_mknod,
            libc::SYS_personality,
            libc::SYS_ustat,
            libc::SYS_statfs,
            libc::SYS_fstatfs,
            libc::SYS_sysfs,
            libc::SYS_getpriority,
            libc::SYS_setpriority,
            libc::SYS_sched_setparam,
            libc::SYS_sched_getparam,
            libc::SYS_sched_setscheduler,
            libc::SYS_sched_getscheduler,
            libc::SYS_sched_get_priority_max,
            libc::SYS_sched_get_priority_min,
            libc::SYS_sched_yield,
            libc::SYS_vhangup,
            libc::SYS_modify_ldt,
            libc::SYS_pivot_root,
            libc::SYS_prctl,
            libc::SYS_arch_prctl,
            libc::SYS_adjtimex,
            libc::SYS_setrlimit,
            libc::SYS_sync,
            libc::SYS_acct,
            libc::SYS_settimeofday,
            libc::SYS_mount,
            libc::SYS_umount2,
            libc::SYS_swapon,
            libc::SYS_swapoff,
            libc::SYS_reboot,
            libc::SYS_sethostname,
            libc::SYS_setdomainname,
            libc::SYS_iopl,
            libc::SYS_ioperm,
            libc::SYS_init_module,
            libc::SYS_delete_module,
            libc::SYS_quotactl,
            libc::SYS_nfsservctl,
            libc::SYS_gettid,
            libc::SYS_readahead,
            libc::SYS_setxattr,
            libc::SYS_lsetxattr,
            libc::SYS_fsetxattr,
            libc::SYS_getxattr,
            libc::SYS_lgetxattr,
            libc::SYS_fgetxattr,
            libc::SYS_listxattr,
            libc::SYS_llistxattr,
            libc::SYS_flistxattr,
            libc::SYS_removexattr,
            libc::SYS_lremovexattr,
            libc::SYS_fremovexattr,
            libc::SYS_tkill,
            libc::SYS_time,
            libc::SYS_futex,
            libc::SYS_sched_setaffinity,
            libc::SYS_sched_getaffinity,
            libc::SYS_io_setup,
            libc::SYS_io_destroy,
            libc::SYS_io_getevents,
            libc::SYS_io_submit,
            libc::SYS_io_cancel,
            libc::SYS_lookup_dcookie,
            libc::SYS_epoll_create,
            libc::SYS_remap_file_pages,
            libc::SYS_set_tid_address,
            libc::SYS_timer_create,
            libc::SYS_timer_settime,
            libc::SYS_timer_gettime,
            libc::SYS_timer_getoverrun,
            libc::SYS_timer_delete,
            libc::SYS_clock_settime,
            libc::SYS_clock_gettime,
            libc::SYS_clock_getres,
            libc::SYS_clock_nanosleep,
            libc::SYS_exit_group,
            libc::SYS_epoll_wait,
            libc::SYS_epoll_ctl,
            libc::SYS_tgkill,
            libc::SYS_utimes,
            libc::SYS_mbind,
            libc::SYS_set_mempolicy,
            libc::SYS_get_mempolicy,
            libc::SYS_mq_open,
            libc::SYS_mq_unlink,
            libc::SYS_mq_timedsend,
            libc::SYS_mq_timedreceive,
            libc::SYS_mq_notify,
            libc::SYS_mq_getsetattr,
            libc::SYS_kexec_load,
            libc::SYS_waitid,
            libc::SYS_add_key,
            libc::SYS_request_key,
            libc::SYS_keyctl,
            libc::SYS_ioprio_set,
            libc::SYS_ioprio_get,
            libc::SYS_inotify_init,
            libc::SYS_inotify_add_watch,
            libc::SYS_inotify_rm_watch,
            libc::SYS_migrate_pages,
            libc::SYS_openat,
            libc::SYS_mkdirat,
            libc::SYS_mknodat,
            libc::SYS_fchownat,
            libc::SYS_futimesat,
            libc::SYS_newfstatat,
            libc::SYS_unlinkat,
            libc::SYS_renameat,
            libc::SYS_linkat,
            libc::SYS_symlinkat,
            libc::SYS_readlinkat,
            libc::SYS_fchmodat,
            libc::SYS_faccessat,
            libc::SYS_pselect6,
            libc::SYS_ppoll,
            libc::SYS_unshare,
            libc::SYS_set_robust_list,
            libc::SYS_get_robust_list,
            libc::SYS_tee,
            libc::SYS_splice,
            libc::SYS_vmsplice,
            libc::SYS_sync_file_range,
            libc::SYS_tee,
            libc::SYS_epoll_pwait,
            libc::SYS_utimensat,
            libc::SYS_timerfd_create,
            libc::SYS_eventfd,
            libc::SYS_fallocate,
            libc::SYS_timerfd_settime,
            libc::SYS_timerfd_gettime,
            libc::SYS_accept4,
            libc::SYS_signalfd,
            libc::SYS_eventfd,
            libc::SYS_dup3,
            libc::SYS_pipe2,
            libc::SYS_inotify_init1,
            libc::SYS_preadv,
            libc::SYS_pwritev,
            libc::SYS_rt_tgsigqueueinfo,
            libc::SYS_perf_event_open,
            libc::SYS_recvmmsg,
            libc::SYS_fanotify_init,
            libc::SYS_fanotify_mark,
            libc::SYS_prlimit64,
            libc::SYS_name_to_handle_at,
            libc::SYS_open_by_handle_at,
            libc::SYS_clock_adjtime,
            libc::SYS_syncfs,
            libc::SYS_sendmmsg,
            libc::SYS_setns,
            libc::SYS_getcpu,
            libc::SYS_process_vm_readv,
            libc::SYS_process_vm_writev,
        ];

        Self { syscalls }
    }

    /// 在非 Linux 系统上创建空的允许列表
    #[cfg(not(target_os = "linux"))]
    pub fn new() -> Self {
        Self { syscalls: vec![] }
    }

    /// 生成 BPF 过滤器代码 (仅 Linux)
    #[cfg(target_os = "linux")]
    pub fn to_bpf_program(&self) -> Vec<libc::sock_filter> {
        use libc::*;
        use std::mem::offset_of;

        let syscalls = &self.syscalls;
        let count = syscalls.len();

        let mut filters: Vec<sock_filter> = Vec::with_capacity(count + 8);

        // 加载系统调用号
        filters.push(sock_filter {
            code: BPF_LD | BPF_W | BPF_ABS,
            jt: 0,
            jf: 0,
            k: offset_of!(sock_filter, code) as u32,
        });

        // 检查是否在允许列表中
        for (i, syscall) in syscalls.iter().enumerate() {
            let jt = if i < count - 1 { 1 } else { 0 };
            let jf = if i == count - 1 { 0 } else { 0 };
            filters.push(sock_filter {
                code: BPF_JMP | BPF_JEQ | BPF_K,
                jt,
                jf,
                k: *syscall as u32,
            });
        }

        // 如果不在列表中则返回 -SECCOMP_RET_KILL
        filters.push(sock_filter {
            code: BPF_RET | BPF_K,
            jt: 0,
            jf: 0,
            k: SECCOMP_RET_KILL as u32,
        });

        // 默认：允许
        filters.push(sock_filter {
            code: BPF_RET | BPF_K,
            jt: 0,
            jf: 0,
            k: SECCOMP_RET_ALLOW as u32,
        });

        filters
    }

    /// 在非 Linux 系统上返回空向量
    #[cfg(not(target_os = "linux"))]
    pub fn to_bpf_program(&self) -> Vec<u8> {
        vec![]
    }
}