# idf_component_register(SRCS "Benchmark.c" "BenchmarkMain2.c"
# INCLUDE_DIRS ".")
idf_component_register(SRCS "main_emfrp_vm.c" "machine.c" "io.c"
    INCLUDE_DIRS "." "include")
