use chin_tools::AResult;

use super::{
    audio::PulseBlock, battery::BatteryBlock, cpu::CpuBlock, memory::MemoryBlock,
    netspeed::NetspeedBlock, temp::TempMonitorBlock, time::TimeBlock, wayland::WaylandBlock,
    Block,
};

pub struct BlockManager {
    pub net_block: NetspeedBlock,
    pub time_block: TimeBlock,
    pub cpu_block: CpuBlock,
    pub battery_block: BatteryBlock,
    pub memory_block: MemoryBlock,
    pub wayland_block: WaylandBlock,
    pub vol_block: PulseBlock,
    pub temp_block: TempMonitorBlock,
}

impl BlockManager {
    pub fn launch() -> AResult<BlockManager> {
        let mut net_block = NetspeedBlock::new();
        net_block.run()?;

        let mut time_block = TimeBlock::new();
        time_block.run()?;

        let mut cpu_block = CpuBlock::new();
        cpu_block.run()?;

        let mut battery_block = BatteryBlock::new()?;
        battery_block.run()?;

        let mut memory_block = MemoryBlock::new();
        memory_block.run()?;

        let mut vol_block = PulseBlock::new();
        vol_block.run()?;

        let mut wayland_block = WaylandBlock::new();
        wayland_block.run()?;

        let mut temp_block = TempMonitorBlock::new();
        temp_block.run()?;

        Ok(BlockManager {
            net_block,
            time_block,
            cpu_block,
            battery_block,
            memory_block,
            wayland_block,
            vol_block,
            temp_block,
        })
    }
}
