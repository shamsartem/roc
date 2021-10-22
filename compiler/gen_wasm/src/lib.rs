mod backend;
pub mod from_wasm32_memory;
mod layout;
mod storage;

#[allow(dead_code)]
pub mod function_builder;

#[allow(dead_code)]
mod opcodes;

use bumpalo::collections::Vec;
use bumpalo::Bump;
use parity_wasm::builder;

use parity_wasm::elements::Internal;
use roc_collections::all::{MutMap, MutSet};
use roc_module::symbol::{Interns, Symbol};
use roc_mono::ir::{Proc, ProcLayout};
use roc_mono::layout::LayoutIds;

use crate::backend::WasmBackend;
use crate::function_builder::{Align, FunctionBuilder, ValueType};

const PTR_SIZE: u32 = 4;
const PTR_TYPE: ValueType = ValueType::I32;

pub const STACK_POINTER_GLOBAL_ID: u32 = 0;
pub const FRAME_ALIGNMENT_BYTES: i32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalId(pub u32);

pub struct Env<'a> {
    pub arena: &'a Bump,
    pub interns: Interns,
    pub exposed_to_host: MutSet<Symbol>,
}

pub fn build_module<'a>(
    env: &'a Env,
    procedures: MutMap<(Symbol, ProcLayout<'a>), Proc<'a>>,
) -> Result<std::vec::Vec<u8>, String> {
    let (builder, _) = build_module_help(env, procedures)?;
    let module = builder.build();
    module
        .into_bytes()
        .map_err(|e| -> String { format!("Error serialising Wasm module {:?}", e) })
}

pub fn build_module_help<'a>(
    env: &'a Env,
    procedures: MutMap<(Symbol, ProcLayout<'a>), Proc<'a>>,
) -> Result<(builder::ModuleBuilder, u32), String> {
    let mut backend = WasmBackend::new(env);
    let mut layout_ids = LayoutIds::default();

    // Sort procedures by occurrence order
    //
    // We sort by the "name", but those are interned strings, and the name that is
    // interned first will have a lower number.
    //
    // But, the name that occurs first is always `main` because it is in the (implicit)
    // file header. Therefore sorting high to low will put other functions before main
    //
    // This means that for now other functions in the file have to be ordered "in reverse": if A
    // uses B, then the name of A must first occur after the first occurrence of the name of B
    let mut procedures = Vec::from_iter_in(procedures.into_iter(), env.arena);
    procedures.sort_by(|a, b| b.0 .0.cmp(&a.0 .0));

    let mut function_index: u32 = 0;
    for ((sym, layout), proc) in procedures {
        function_index = backend.build_proc(proc, sym)?;
        if env.exposed_to_host.contains(&sym) {
            let fn_name = layout_ids
                .get_toplevel(sym, &layout)
                .to_symbol_string(sym, &env.interns);

            let export = builder::export()
                .field(fn_name.as_str())
                .with_internal(Internal::Function(function_index))
                .build();

            backend.module_builder.push_export(export);
        }
    }

    // Because of the sorting above, we know the last function in the `for` is the main function.
    // Here we grab its index and return it, so that the test_wrapper is able to call it.
    // This is a workaround until we implement object files with symbols and relocations.
    let main_function_index = function_index;

    const MIN_MEMORY_SIZE_KB: u32 = 1024;
    const PAGE_SIZE_KB: u32 = 64;

    let memory = builder::MemoryBuilder::new()
        .with_min(MIN_MEMORY_SIZE_KB / PAGE_SIZE_KB)
        .build();
    backend.module_builder.push_memory(memory);
    let memory_export = builder::export()
        .field("memory")
        .with_internal(Internal::Memory(0))
        .build();
    backend.module_builder.push_export(memory_export);

    let stack_pointer_global = builder::global()
        .with_type(parity_wasm::elements::ValueType::I32)
        .mutable()
        .init_expr(parity_wasm::elements::Instruction::I32Const(
            (MIN_MEMORY_SIZE_KB * 1024) as i32,
        ))
        .build();
    backend.module_builder.push_global(stack_pointer_global);

    Ok((backend.module_builder, main_function_index))
}

fn encode_alignment(bytes: u32) -> Align {
    match bytes {
        1 => Align::Bytes1,
        2 => Align::Bytes2,
        4 => Align::Bytes4,
        8 => Align::Bytes8,
        _ => panic!("{:?}-byte alignment is not supported", bytes),
    }
}

pub struct CopyMemoryConfig {
    from_ptr: LocalId,
    from_offset: u32,
    to_ptr: LocalId,
    to_offset: u32,
    size: u32,
    alignment_bytes: u32,
}

pub fn copy_memory(code_builder: &mut FunctionBuilder, config: CopyMemoryConfig) {
    if config.from_ptr == config.to_ptr && config.from_offset == config.to_offset {
        return;
    }

    let alignment = encode_alignment(config.alignment_bytes);
    let mut i = 0;
    while config.size - i >= 8 {
        code_builder.get_local(config.to_ptr);
        code_builder.get_local(config.from_ptr);
        code_builder.i64_load(alignment, i + config.from_offset);
        code_builder.i64_store(alignment, i + config.to_offset);
        i += 8;
    }
    if config.size - i >= 4 {
        code_builder.get_local(config.to_ptr);
        code_builder.get_local(config.from_ptr);
        code_builder.i32_load(alignment, i + config.from_offset);
        code_builder.i32_store(alignment, i + config.to_offset);
        i += 4;
    }
    while config.size - i > 0 {
        code_builder.get_local(config.to_ptr);
        code_builder.get_local(config.from_ptr);
        code_builder.i32_load8_u(alignment, i + config.from_offset);
        code_builder.i32_store8(alignment, i + config.to_offset);
        i += 1;
    }
}

/// Round up to alignment_bytes (which must be a power of 2)
pub fn round_up_to_alignment(unaligned: i32, alignment_bytes: i32) -> i32 {
    debug_assert!(alignment_bytes.count_ones() == 1);
    let mut aligned = unaligned;
    aligned += alignment_bytes - 1; // if lower bits are non-zero, push it over the next boundary
    aligned &= -alignment_bytes; // mask with a flag that has upper bits 1, lower bits 0
    aligned
}

pub fn debug_panic<E: std::fmt::Debug>(error: E) {
    panic!("{:?}", error);
}

/// Write a u32 value as LEB-128 encoded bytes into the provided buffer, returning byte length
///
/// All integers in Wasm are variable-length encoded, which saves space for small values.
/// The most significant bit indicates "more bytes are coming", and the other 7 are payload.
pub fn encode_u32<'a>(buffer: &mut [u8], mut value: u32) -> usize {
    let mut count = 0;
    while value >= 0x80 {
        buffer[count] = 0x80 | ((value & 0x7f) as u8);
        value >>= 7;
        count += 1;
    }
    buffer[count] = value as u8;
    count + 1
}

/// Write a u64 value as LEB-128 encoded bytes, into the provided buffer, returning byte length
///
/// All integers in Wasm are variable-length encoded, which saves space for small values.
/// The most significant bit indicates "more bytes are coming", and the other 7 are payload.
pub fn encode_u64<'a>(buffer: &mut Vec<'a, u8>, mut value: u64) -> usize {
    let mut count = 0;
    while value >= 0x80 {
        buffer[count] = 0x80 | ((value & 0x7f) as u8);
        value >>= 7;
        count += 1;
    }
    buffer[count] = value as u8;
    count + 1
}
