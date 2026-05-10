use std::borrow::Cow;

use crate::spec::{
    Arch, Cc, CodeModel, Env, LinkerFlavor, Lld, LlvmAbi, Os, PanicStrategy, RelocModel,
    Target, TargetMetadata, TargetOptions, cvs,
};

const EXPORT_SYMBOLS: &[&str] = &["main"];

pub(crate) fn target() -> Target {
    let mut options = TargetOptions {
        families: cvs!["unix"],
        os: Os::Badgevms,
        env: Env::Unspecified,
        vendor: "unknown".into(),
        linker_flavor: LinkerFlavor::Gnu(Cc::No, Lld::Yes),
        linker: Some("rust-lld".into()),
        cpu: "generic-rv32".into(),
        code_model: Some(CodeModel::Medium),
        max_atomic_width: Some(32),
        atomic_cas: true,
        llvm_abiname: LlvmAbi::Ilp32f,
        features: "+m,+a,+c,+f".into(),
        panic_strategy: PanicStrategy::Abort,
        relocation_model: RelocModel::Pic,
        override_export_symbols: Some(EXPORT_SYMBOLS.iter().cloned().map(Cow::from).collect()),
        emit_debug_gdb_scripts: false,
        eh_frame_header: false,
        disable_redzone: true,
        dynamic_linking: true,
        executables: true,
        ..Default::default()
    };

    // BadgeVMS applications are dynamically loaded ET_DYN objects. There is no
    // target-side C runtime or startup object; the loader calls the exported
    // `main` entry point directly and resolves BadgeVMS imports at load time.
    options.add_pre_link_args(
        LinkerFlavor::Gnu(Cc::No, Lld::No),
        &["--shared", "--entry=main", "--gc-sections", "--discard-locals"],
    );

    Target {
        data_layout: "e-m:e-p:32:32-i64:64-n32-S128".into(),
        llvm_target: "riscv32".into(),
        metadata: TargetMetadata {
            description: Some("RISC-V BadgeVMS (RV32IMAFC ISA)".into()),
            tier: Some(3),
            host_tools: Some(false),
            std: Some(true),
        },
        pointer_width: 32,
        arch: Arch::RiscV32,
        options,
    }
}