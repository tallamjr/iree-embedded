# CMake toolchain for the IREE runtime on bare-metal Cortex-M4F (nRF52833 /
# BBC micro:bit v2). Modelled on IREE's generic_riscv32.cmake.

cmake_minimum_required(VERSION 3.26)
if(ARM_CM_TOOLCHAIN_INCLUDED)
  return()
endif()
set(ARM_CM_TOOLCHAIN_INCLUDED true)

set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR arm)
# Build static libraries only; avoid needing a full link environment for the
# compiler probe (no startup files / linker script here).
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)

set(CMAKE_C_COMPILER arm-none-eabi-gcc)
set(CMAKE_CXX_COMPILER arm-none-eabi-g++)
set(CMAKE_ASM_COMPILER arm-none-eabi-gcc)
set(CMAKE_AR arm-none-eabi-ar)
set(CMAKE_RANLIB arm-none-eabi-ranlib)
set(CMAKE_CROSSCOMPILING ON CACHE BOOL "")

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_EXTENSIONS OFF)

# Bare-metal runtime configuration (single-threaded, no OS, no file IO).
set(IREE_HAL_DRIVER_DEFAULTS OFF CACHE BOOL "" FORCE)
set(IREE_HAL_DRIVER_LOCAL_SYNC ON CACHE BOOL "" FORCE)
set(IREE_HAL_EXECUTABLE_LOADER_DEFAULTS OFF CACHE BOOL "" FORCE)
set(IREE_HAL_EXECUTABLE_LOADER_EMBEDDED_ELF ON CACHE BOOL "" FORCE)
set(IREE_HAL_EXECUTABLE_LOADER_VMVX_MODULE ON CACHE BOOL "" FORCE)
set(IREE_HAL_EXECUTABLE_PLUGIN_DEFAULTS OFF CACHE BOOL "" FORCE)
set(IREE_HAL_EXECUTABLE_PLUGIN_EMBEDDED_ELF ON CACHE BOOL "" FORCE)
set(IREE_ENABLE_THREADING OFF CACHE BOOL "" FORCE)
set(IREE_SYNCHRONIZATION_DISABLE_UNSAFE ON CACHE BOOL "" FORCE)
set(IREE_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(IREE_BUILD_COMPILER OFF CACHE BOOL "" FORCE)

set(ARM_CM_FLAGS "\
    -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -ffreestanding \
    -DIREE_PLATFORM_GENERIC=1 -DIREE_FILE_IO_ENABLE=0 \
    -DIREE_TIME_NOW_FN=\"\{ return 0; \}\" \
    -DIREE_DEVICE_SIZE_T=uint32_t -DPRIdsz=PRIu32")

set(CMAKE_C_FLAGS   "${ARM_CM_FLAGS} ${CMAKE_C_FLAGS}")
set(CMAKE_CXX_FLAGS "${ARM_CM_FLAGS} ${CMAKE_CXX_FLAGS}")
set(CMAKE_ASM_FLAGS "${ARM_CM_FLAGS} ${CMAKE_ASM_FLAGS}")
