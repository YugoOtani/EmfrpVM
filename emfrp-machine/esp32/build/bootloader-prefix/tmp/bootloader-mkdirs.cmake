# Distributed under the OSI-approved BSD 3-Clause License.  See accompanying
# file Copyright.txt or https://cmake.org/licensing for details.

cmake_minimum_required(VERSION 3.5)

file(MAKE_DIRECTORY
  "/Users/yugootani/esp/esp-idf/components/bootloader/subproject"
  "/Users/yugootani/Documents/GitHub/Emfrp-VM/emfrp-machine/esp32/build/bootloader"
  "/Users/yugootani/Documents/GitHub/Emfrp-VM/emfrp-machine/esp32/build/bootloader-prefix"
  "/Users/yugootani/Documents/GitHub/Emfrp-VM/emfrp-machine/esp32/build/bootloader-prefix/tmp"
  "/Users/yugootani/Documents/GitHub/Emfrp-VM/emfrp-machine/esp32/build/bootloader-prefix/src/bootloader-stamp"
  "/Users/yugootani/Documents/GitHub/Emfrp-VM/emfrp-machine/esp32/build/bootloader-prefix/src"
  "/Users/yugootani/Documents/GitHub/Emfrp-VM/emfrp-machine/esp32/build/bootloader-prefix/src/bootloader-stamp"
)

set(configSubDirs )
foreach(subDir IN LISTS configSubDirs)
    file(MAKE_DIRECTORY "/Users/yugootani/Documents/GitHub/Emfrp-VM/emfrp-machine/esp32/build/bootloader-prefix/src/bootloader-stamp/${subDir}")
endforeach()
if(cfgdir)
  file(MAKE_DIRECTORY "/Users/yugootani/Documents/GitHub/Emfrp-VM/emfrp-machine/esp32/build/bootloader-prefix/src/bootloader-stamp${cfgdir}") # cfgdir has leading slash
endif()
