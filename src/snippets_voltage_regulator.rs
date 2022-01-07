/* 
//Build for rtic with device being the nrf52840 pac

// Set internal regulator voltage to 3v3 instead of 1v8
if !ctx.device.UICR.regout0.read().vout().is_3v3() {
    // Enable erase
    ctx.device.NVMC.config.write(|w| {
        w.wen().een()
    });
    while ctx.device.NVMC.ready.read().ready().is_busy() {}

    // Erase regout0 page
    ctx.device.NVMC.erasepage().write(|w| unsafe {
        w.erasepage().bits(&ctx.device.UICR.regout0 as *const _ as u32)
    });
    while ctx.device.NVMC.ready.read().ready().is_busy() {}

    // enable write
    ctx.device.NVMC.config.write(|w| {
        w.wen().wen()
    });
    while ctx.device.NVMC.ready.read().ready().is_busy() {}

    // Set 3v3 setting
    ctx.device.UICR.regout0.write(|w| {
        w.vout()._3v3()
    });
    while ctx.device.NVMC.ready.read().ready().is_busy() {}

    // Return UCIR to read only
    ctx.device.NVMC.config.write(|w| {
        w.wen().ren()
    });
    while ctx.device.NVMC.ready.read().ready().is_busy() {}

    // system reset
    SCB::sys_reset();
}

 */