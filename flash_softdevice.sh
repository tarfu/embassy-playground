#!/bin/sh

probe-rs-cli erase --chip nrf52840
probe-rs-cli download --chip nrf52840 --format hex s140_nrf52_7.3.0_softdevice.hex
