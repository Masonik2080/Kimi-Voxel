# Kimi-Voxel

A voxel game engine written in Rust with GPU-accelerated rendering.

## Features

- Procedural terrain generation with biomes
- GPU-based voxel rendering with WGPU
- Cascaded shadow maps
- Subvoxel detail system
- Audio system with footsteps, jumps, and block placement sounds
- Save/load world system
- Inventory and hotbar UI

## Requirements

- Rust 1.70+
- GPU with Vulkan/Metal/DX12 support

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

## Controls

- WASD - Movement
- Space - Jump
- Mouse - Look around
- Left Click - Break block
- Right Click - Place block

## License

MIT
