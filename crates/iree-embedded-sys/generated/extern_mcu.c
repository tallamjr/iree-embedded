#include "wrapper.h"

// Static wrappers

iree_host_size_t iree_host_align__extern(iree_host_size_t value, iree_host_size_t alignment) { return iree_host_align(value, alignment); }
bool iree_host_size_is_power_of_two__extern(iree_host_size_t value) { return iree_host_size_is_power_of_two(value); }
bool iree_host_size_is_valid_alignment__extern(iree_host_size_t alignment) { return iree_host_size_is_valid_alignment(alignment); }
bool iree_host_size_has_alignment__extern(iree_host_size_t value, iree_host_size_t alignment) { return iree_host_size_has_alignment(value, alignment); }
iree_host_size_t iree_host_size_next_power_of_two__extern(iree_host_size_t value) { return iree_host_size_next_power_of_two(value); }
iree_device_size_t iree_device_align__extern(iree_device_size_t value, iree_device_size_t alignment) { return iree_device_align(value, alignment); }
bool iree_device_size_is_power_of_two__extern(iree_device_size_t value) { return iree_device_size_is_power_of_two(value); }
bool iree_device_size_is_valid_alignment__extern(iree_device_size_t alignment) { return iree_device_size_is_valid_alignment(alignment); }
bool iree_device_size_has_alignment__extern(iree_device_size_t value, iree_device_size_t alignment) { return iree_device_size_has_alignment(value, alignment); }
iree_device_size_t iree_device_size_next_power_of_two__extern(iree_device_size_t value) { return iree_device_size_next_power_of_two(value); }
bool iree_is_power_of_two_uint64__extern(uint64_t value) { return iree_is_power_of_two_uint64(value); }
uint64_t iree_align_uint64__extern(uint64_t value, uint64_t alignment) { return iree_align_uint64(value, alignment); }
iree_host_size_t iree_host_size_ceil_div__extern(iree_host_size_t lhs, iree_host_size_t rhs) { return iree_host_size_ceil_div(lhs, rhs); }
iree_host_size_t iree_host_size_floor_div__extern(iree_host_size_t lhs, iree_host_size_t rhs) { return iree_host_size_floor_div(lhs, rhs); }
iree_device_size_t iree_device_size_ceil_div__extern(iree_device_size_t lhs, iree_device_size_t rhs) { return iree_device_size_ceil_div(lhs, rhs); }
iree_device_size_t iree_device_size_floor_div__extern(iree_device_size_t lhs, iree_device_size_t rhs) { return iree_device_size_floor_div(lhs, rhs); }
iree_device_size_t iree_device_size_gcd__extern(iree_device_size_t a, iree_device_size_t b) { return iree_device_size_gcd(a, b); }
iree_device_size_t iree_device_size_lcm__extern(iree_device_size_t a, iree_device_size_t b) { return iree_device_size_lcm(a, b); }
uintptr_t iree_page_align_start__extern(uintptr_t addr, iree_host_size_t page_alignment) { return iree_page_align_start(addr, page_alignment); }
uintptr_t iree_page_align_end__extern(uintptr_t addr, iree_host_size_t page_alignment) { return iree_page_align_end(addr, page_alignment); }
iree_page_range_t iree_page_range_union__extern(const iree_page_range_t a, const iree_page_range_t b) { return iree_page_range_union(a, b); }
iree_page_range_t iree_align_byte_range_to_pages__extern(const iree_byte_range_t byte_range, iree_host_size_t page_alignment) { return iree_align_byte_range_to_pages(byte_range, page_alignment); }
void iree_page_align_range__extern(void *base_address, iree_byte_range_t range, iree_host_size_t page_alignment, void **out_start_address, iree_host_size_t *out_aligned_length) { iree_page_align_range(base_address, range, page_alignment, out_start_address, out_aligned_length); }
uint8_t iree_unaligned_load_le_u8__extern(const uint8_t *ptr) { return iree_unaligned_load_le_u8(ptr); }
void iree_unaligned_store_le_u8__extern(uint8_t *ptr, uint8_t value) { iree_unaligned_store_le_u8(ptr, value); }
uint16_t iree_unaligned_load_le_u16__extern(const uint16_t *ptr) { return iree_unaligned_load_le_u16(ptr); }
void iree_unaligned_store_le_u16__extern(uint16_t *ptr, uint16_t value) { iree_unaligned_store_le_u16(ptr, value); }
uint32_t iree_unaligned_load_le_u32__extern(const uint32_t *ptr) { return iree_unaligned_load_le_u32(ptr); }
float iree_unaligned_load_le_f32__extern(const float *ptr) { return iree_unaligned_load_le_f32(ptr); }
void iree_unaligned_store_le_u32__extern(uint32_t *ptr, uint32_t value) { iree_unaligned_store_le_u32(ptr, value); }
void iree_unaligned_store_le_f32__extern(float *ptr, float value) { iree_unaligned_store_le_f32(ptr, value); }
uint64_t iree_unaligned_load_le_u64__extern(const uint64_t *ptr) { return iree_unaligned_load_le_u64(ptr); }
double iree_unaligned_load_le_f64__extern(const double *ptr) { return iree_unaligned_load_le_f64(ptr); }
void iree_unaligned_store_le_u64__extern(uint64_t *ptr, uint64_t value) { iree_unaligned_store_le_u64(ptr, value); }
void iree_unaligned_store_le_f64__extern(double *ptr, double value) { iree_unaligned_store_le_f64(ptr, value); }
iree_string_view_t iree_string_view_empty__extern(void) { return iree_string_view_empty(); }
iree_string_view_t iree_make_string_view__extern(const char *str, iree_host_size_t str_length) { return iree_make_string_view(str, str_length); }
iree_string_view_t iree_make_cstring_view__extern(const char *str) { return iree_make_cstring_view(str); }
iree_mutable_string_view_t iree_mutable_string_view_empty__extern(void) { return iree_mutable_string_view_empty(); }
iree_mutable_string_view_t iree_make_mutable_string_view__extern(char *str, iree_host_size_t str_length) { return iree_make_mutable_string_view(str, str_length); }
iree_string_view_t iree_make_const_string_view__extern(iree_mutable_string_view_t mutable_view) { return iree_make_const_string_view(mutable_view); }
iree_string_pair_t iree_string_pair_empty__extern(void) { return iree_string_pair_empty(); }
iree_string_pair_t iree_make_string_pair__extern(iree_string_view_t first, iree_string_view_t second) { return iree_make_string_pair(first, second); }
iree_string_pair_t iree_make_cstring_pair__extern(const char *first, const char *second) { return iree_make_cstring_pair(first, second); }
iree_string_pair_list_t iree_string_pair_list_empty__extern(void) { return iree_string_pair_list_empty(); }
iree_string_view_list_t iree_string_view_list_empty__extern(void) { return iree_string_view_list_empty(); }
bool iree_string_view_consume_prefix_char__extern(iree_string_view_t *value, char c) { return iree_string_view_consume_prefix_char(value, c); }
bool iree_string_view_starts_with_char__extern(iree_string_view_t value, char c) { return iree_string_view_starts_with_char(value, c); }
iree_byte_span_t iree_make_byte_span__extern(void *data, iree_host_size_t data_length) { return iree_make_byte_span(data, data_length); }
iree_byte_span_t iree_byte_span_empty__extern(void) { return iree_byte_span_empty(); }
bool iree_byte_span_is_empty__extern(iree_byte_span_t span) { return iree_byte_span_is_empty(span); }
iree_const_byte_span_t iree_make_const_byte_span__extern(const void *data, iree_host_size_t data_length) { return iree_make_const_byte_span(data, data_length); }
iree_const_byte_span_t iree_const_byte_span_empty__extern(void) { return iree_const_byte_span_empty(); }
bool iree_const_byte_span_is_empty__extern(iree_const_byte_span_t span) { return iree_const_byte_span_is_empty(span); }
iree_const_byte_span_t iree_const_cast_byte_span__extern(iree_byte_span_t span) { return iree_const_cast_byte_span(span); }
iree_byte_span_t iree_cast_const_byte_span__extern(iree_const_byte_span_t span) { return iree_cast_const_byte_span(span); }
void iree_memcpy_stream_dst__extern(void *dst, const void *src, iree_host_size_t size) { iree_memcpy_stream_dst(dst, src, size); }
bool iree_host_size_checked_add__extern(iree_host_size_t a, iree_host_size_t b, iree_host_size_t *out_result) { return iree_host_size_checked_add(a, b, out_result); }
bool iree_host_size_checked_mul__extern(iree_host_size_t a, iree_host_size_t b, iree_host_size_t *out_result) { return iree_host_size_checked_mul(a, b, out_result); }
bool iree_host_size_checked_mul_add__extern(iree_host_size_t base, iree_host_size_t count, iree_host_size_t element_size, iree_host_size_t *out_result) { return iree_host_size_checked_mul_add(base, count, element_size, out_result); }
bool iree_device_size_checked_add__extern(iree_device_size_t a, iree_device_size_t b, iree_device_size_t *out_result) { return iree_device_size_checked_add(a, b, out_result); }
bool iree_device_size_checked_mul__extern(iree_device_size_t a, iree_device_size_t b, iree_device_size_t *out_result) { return iree_device_size_checked_mul(a, b, out_result); }
bool iree_device_size_checked_mul_add__extern(iree_device_size_t base, iree_device_size_t count, iree_device_size_t element_size, iree_device_size_t *out_result) { return iree_device_size_checked_mul_add(base, count, element_size, out_result); }
bool iree_host_size_checked_align__extern(iree_host_size_t value, iree_host_size_t alignment, iree_host_size_t *out_aligned) { return iree_host_size_checked_align(value, alignment, out_aligned); }
bool iree_device_size_checked_align__extern(iree_device_size_t value, iree_device_size_t alignment, iree_device_size_t *out_aligned) { return iree_device_size_checked_align(value, alignment, out_aligned); }
iree_status_t iree_struct_layout_calculate__extern(iree_host_size_t base_size, const iree_struct_field_t *fields, iree_host_size_t field_count, iree_host_size_t *out_total) { return iree_struct_layout_calculate(base_size, fields, field_count, out_total); }
iree_allocator_t iree_allocator_null__extern(void) { return iree_allocator_null(); }
bool iree_allocator_is_null__extern(iree_allocator_t allocator) { return iree_allocator_is_null(allocator); }
iree_allocator_t iree_allocator_inline_arena__extern(iree_allocator_inline_storage_t *storage) { return iree_allocator_inline_arena(storage); }
void iree_abort__extern(void) { iree_abort(); }
iree_string_pair_t * iree_string_pair_builder_pairs__extern(iree_string_pair_builder_t *builder) { return iree_string_pair_builder_pairs(builder); }
iree_host_size_t iree_string_pair_builder_size__extern(iree_string_pair_builder_t *builder) { return iree_string_pair_builder_size(builder); }
iree_duration_t iree_make_duration_ms__extern(int64_t timeout_ms) { return iree_make_duration_ms(timeout_ms); }
iree_timeout_t iree_immediate_timeout__extern(void) { return iree_immediate_timeout(); }
bool iree_timeout_is_immediate__extern(iree_timeout_t timeout) { return iree_timeout_is_immediate(timeout); }
iree_timeout_t iree_infinite_timeout__extern(void) { return iree_infinite_timeout(); }
bool iree_timeout_is_infinite__extern(iree_timeout_t timeout) { return iree_timeout_is_infinite(timeout); }
iree_timeout_t iree_make_deadline__extern(iree_time_t deadline_ns) { return iree_make_deadline(deadline_ns); }
iree_timeout_t iree_make_timeout_ns__extern(iree_duration_t timeout_ns) { return iree_make_timeout_ns(timeout_ns); }
iree_timeout_t iree_make_timeout_ms__extern(iree_duration_t timeout_ms) { return iree_make_timeout_ms(timeout_ms); }
void iree_convert_timeout_to_absolute__extern(iree_timeout_t *timeout) { iree_convert_timeout_to_absolute(timeout); }
iree_time_t iree_timeout_as_deadline_ns__extern(iree_timeout_t timeout) { return iree_timeout_as_deadline_ns(timeout); }
iree_duration_t iree_timeout_as_duration_ns__extern(iree_timeout_t timeout) { return iree_timeout_as_duration_ns(timeout); }
iree_timeout_t iree_timeout_min__extern(iree_timeout_t lhs, iree_timeout_t rhs) { return iree_timeout_min(lhs, rhs); }
iree_wait_primitive_t iree_make_wait_primitive__extern(iree_wait_primitive_type_t type, iree_wait_primitive_value_t value) { return iree_make_wait_primitive(type, value); }
iree_wait_primitive_t iree_wait_primitive_immediate__extern(void) { return iree_wait_primitive_immediate(); }
bool iree_wait_primitive_is_immediate__extern(iree_wait_primitive_t wait_primitive) { return iree_wait_primitive_is_immediate(wait_primitive); }
iree_wait_source_t iree_wait_source_immediate__extern(void) { return iree_wait_source_immediate(); }
bool iree_wait_source_is_immediate__extern(iree_wait_source_t wait_source) { return iree_wait_source_is_immediate(wait_source); }
iree_wait_source_t iree_wait_source_delay__extern(iree_time_t deadline_ns) { return iree_wait_source_delay(deadline_ns); }
bool iree_wait_source_is_delay__extern(iree_wait_source_t wait_source) { return iree_wait_source_is_delay(wait_source); }
iree_loop_t iree_loop_null__extern(void) { return iree_loop_null(); }
iree_loop_t iree_loop_inline__extern(iree_status_t *out_status) { return iree_loop_inline(out_status); }
iree_loop_t iree_loop_inline_initialize__extern(iree_loop_inline_storage_t *storage) { return iree_loop_inline_initialize(storage); }
void iree_loop_inline_deinitialize__extern(iree_loop_inline_storage_t *storage) { iree_loop_inline_deinitialize(storage); }
void * iree_tracing_obscure_ptr__extern(void *ptr) { return iree_tracing_obscure_ptr(ptr); }
void iree_hal_resource_initialize__extern(const void *vtable, iree_hal_resource_t *out_resource) { iree_hal_resource_initialize(vtable, out_resource); }
void iree_hal_resource_retain__extern(const void *any_resource) { iree_hal_resource_retain(any_resource); }
void iree_hal_resource_release__extern(const void *any_resource) { iree_hal_resource_release(any_resource); }
bool iree_hal_resource_is__extern(const void *resource, const void *vtable) { return iree_hal_resource_is(resource, vtable); }
iree_hal_buffer_placement_t iree_hal_buffer_placement_undefined__extern(void) { return iree_hal_buffer_placement_undefined(); }
bool iree_hal_buffer_placement_is_undefined__extern(const iree_hal_buffer_placement_t placement) { return iree_hal_buffer_placement_is_undefined(placement); }
void iree_hal_buffer_params_canonicalize__extern(iree_hal_buffer_params_t *params) { iree_hal_buffer_params_canonicalize(params); }
iree_hal_buffer_params_t iree_hal_buffer_params_with_usage__extern(const iree_hal_buffer_params_t params, iree_hal_buffer_usage_t usage) { return iree_hal_buffer_params_with_usage(params, usage); }
iree_hal_buffer_release_callback_t iree_hal_buffer_release_callback_null__extern(void) { return iree_hal_buffer_release_callback_null(); }
void iree_hal_allocator_statistics_record_alloc__extern(iree_hal_allocator_statistics_t *statistics, iree_hal_memory_type_t memory_type, iree_device_size_t allocation_size) { iree_hal_allocator_statistics_record_alloc(statistics, memory_type, allocation_size); }
void iree_hal_allocator_statistics_record_free__extern(iree_hal_allocator_statistics_t *statistics, iree_hal_memory_type_t memory_type, iree_device_size_t allocation_size) { iree_hal_allocator_statistics_record_free(statistics, memory_type, allocation_size); }
iree_hal_buffer_ref_t iree_hal_make_buffer_ref__extern(iree_hal_buffer_t *buffer, iree_device_size_t offset, iree_device_size_t length) { return iree_hal_make_buffer_ref(buffer, offset, length); }
iree_hal_buffer_ref_t iree_hal_make_indirect_buffer_ref__extern(uint32_t buffer_slot, iree_device_size_t offset, iree_device_size_t length) { return iree_hal_make_indirect_buffer_ref(buffer_slot, offset, length); }
iree_hal_buffer_ref_list_t iree_hal_buffer_ref_list_empty__extern(void) { return iree_hal_buffer_ref_list_empty(); }
iree_hal_dispatch_config_t iree_hal_make_static_dispatch_config__extern(uint32_t workgroup_count_x, uint32_t workgroup_count_y, uint32_t workgroup_count_z) { return iree_hal_make_static_dispatch_config(workgroup_count_x, workgroup_count_y, workgroup_count_z); }
bool iree_hal_dispatch_uses_indirect_parameters__extern(iree_hal_dispatch_flags_t flags) { return iree_hal_dispatch_uses_indirect_parameters(flags); }
bool iree_hal_dispatch_uses_custom_arguments__extern(iree_hal_dispatch_flags_t flags) { return iree_hal_dispatch_uses_custom_arguments(flags); }
bool iree_hal_dispatch_uses_indirect_arguments__extern(iree_hal_dispatch_flags_t flags) { return iree_hal_dispatch_uses_indirect_arguments(flags); }
iree_hal_label_color_t iree_hal_label_color_unspecified__extern(void) { return iree_hal_label_color_unspecified(); }
iree_hal_buffer_binding_table_t iree_hal_buffer_binding_table_empty__extern(void) { return iree_hal_buffer_binding_table_empty(); }
bool iree_hal_buffer_binding_table_is_empty__extern(iree_hal_buffer_binding_table_t binding_table) { return iree_hal_buffer_binding_table_is_empty(binding_table); }
iree_status_t iree_hal_buffer_binding_table_resolve_ref__extern(iree_hal_buffer_binding_table_t binding_table, iree_hal_buffer_ref_t buffer_ref, iree_hal_buffer_ref_t *out_resolved_ref) { return iree_hal_buffer_binding_table_resolve_ref(binding_table, buffer_ref, out_resolved_ref); }
uint64_t iree_hal_status_as_semaphore_failure__extern(iree_status_t status) { return iree_hal_status_as_semaphore_failure(status); }
iree_status_t iree_hal_semaphore_failure_as_status__extern(uint64_t value) { return iree_hal_semaphore_failure_as_status(value); }
void iree_hal_semaphore_failure_free__extern(uint64_t value) { iree_hal_semaphore_failure_free(value); }
iree_hal_semaphore_list_t iree_hal_semaphore_list_empty__extern(void) { return iree_hal_semaphore_list_empty(); }
bool iree_hal_semaphore_list_is_empty__extern(iree_hal_semaphore_list_t semaphore_list) { return iree_hal_semaphore_list_is_empty(semaphore_list); }
iree_io_file_handle_release_callback_t iree_io_file_handle_release_callback_null__extern(void) { return iree_io_file_handle_release_callback_null(); }
iree_io_file_handle_type_t iree_io_file_handle_type__extern(const iree_io_file_handle_t *handle) { return iree_io_file_handle_type(handle); }
iree_io_file_handle_primitive_value_t iree_io_file_handle_value__extern(const iree_io_file_handle_t *handle) { return iree_io_file_handle_value(handle); }
iree_hal_topology_edge_t iree_hal_topology_edge_empty__extern(void) { return iree_hal_topology_edge_empty(); }
bool iree_hal_topology_edge_is_empty__extern(iree_hal_topology_edge_t edge) { return iree_hal_topology_edge_is_empty(edge); }
iree_hal_resource_origin_t iree_hal_resource_origin_undefined__extern(void) { return iree_hal_resource_origin_undefined(); }
iree_hal_topology_t iree_hal_topology_empty__extern(void) { return iree_hal_topology_empty(); }
bool iree_hal_topology_is_empty__extern(const iree_hal_topology_t *topology) { return iree_hal_topology_is_empty(topology); }
uint32_t iree_hal_topology_device_count__extern(const iree_hal_topology_t *topology) { return iree_hal_topology_device_count(topology); }
iree_hal_topology_edge_t iree_hal_topology_query_edge__extern(const iree_hal_topology_t *topology, uint32_t src_ordinal, uint32_t dst_ordinal) { return iree_hal_topology_query_edge(topology, src_ordinal, dst_ordinal); }
iree_hal_topology_interop_mode_t iree_hal_topology_edge_wait_mode__extern(iree_hal_topology_edge_scheduling_word_t word) { return iree_hal_topology_edge_wait_mode(word); }
iree_hal_topology_interop_mode_t iree_hal_topology_edge_signal_mode__extern(iree_hal_topology_edge_scheduling_word_t word) { return iree_hal_topology_edge_signal_mode(word); }
iree_hal_topology_interop_mode_t iree_hal_topology_edge_buffer_read_mode__extern(iree_hal_topology_edge_scheduling_word_t word) { return iree_hal_topology_edge_buffer_read_mode(word); }
iree_hal_topology_interop_mode_t iree_hal_topology_edge_buffer_write_mode__extern(iree_hal_topology_edge_scheduling_word_t word) { return iree_hal_topology_edge_buffer_write_mode(word); }
iree_hal_topology_capability_t iree_hal_topology_edge_capability_flags__extern(iree_hal_topology_edge_scheduling_word_t word) { return iree_hal_topology_edge_capability_flags(word); }
uint8_t iree_hal_topology_edge_wait_cost__extern(iree_hal_topology_edge_scheduling_word_t word) { return iree_hal_topology_edge_wait_cost(word); }
uint8_t iree_hal_topology_edge_signal_cost__extern(iree_hal_topology_edge_scheduling_word_t word) { return iree_hal_topology_edge_signal_cost(word); }
uint8_t iree_hal_topology_edge_copy_cost__extern(iree_hal_topology_edge_scheduling_word_t word) { return iree_hal_topology_edge_copy_cost(word); }
uint8_t iree_hal_topology_edge_latency_class__extern(iree_hal_topology_edge_scheduling_word_t word) { return iree_hal_topology_edge_latency_class(word); }
uint8_t iree_hal_topology_edge_numa_distance__extern(iree_hal_topology_edge_scheduling_word_t word) { return iree_hal_topology_edge_numa_distance(word); }
iree_hal_topology_link_class_t iree_hal_topology_edge_link_class__extern(iree_hal_topology_edge_scheduling_word_t word) { return iree_hal_topology_edge_link_class(word); }
iree_hal_topology_handle_type_t iree_hal_topology_edge_semaphore_import_types__extern(iree_hal_topology_edge_interop_word_t word) { return iree_hal_topology_edge_semaphore_import_types(word); }
iree_hal_topology_handle_type_t iree_hal_topology_edge_semaphore_export_types__extern(iree_hal_topology_edge_interop_word_t word) { return iree_hal_topology_edge_semaphore_export_types(word); }
iree_hal_topology_handle_type_t iree_hal_topology_edge_buffer_import_types__extern(iree_hal_topology_edge_interop_word_t word) { return iree_hal_topology_edge_buffer_import_types(word); }
iree_hal_topology_handle_type_t iree_hal_topology_edge_buffer_export_types__extern(iree_hal_topology_edge_interop_word_t word) { return iree_hal_topology_edge_buffer_export_types(word); }
iree_hal_host_call_t iree_hal_make_host_call__extern(iree_hal_host_call_fn_t fn, void *user_data) { return iree_hal_make_host_call(fn, user_data); }
iree_hal_topology_edge_t iree_hal_device_topology_query_edge__extern(const iree_hal_device_topology_info_t *src_info, const iree_hal_device_topology_info_t *dst_info) { return iree_hal_device_topology_query_edge(src_info, dst_info); }
iree_hal_transfer_buffer_t iree_hal_make_host_transfer_buffer__extern(iree_byte_span_t host_buffer) { return iree_hal_make_host_transfer_buffer(host_buffer); }
iree_hal_transfer_buffer_t iree_hal_make_host_transfer_buffer_span__extern(void *ptr, iree_host_size_t length) { return iree_hal_make_host_transfer_buffer_span(ptr, length); }
iree_hal_transfer_buffer_t iree_hal_make_device_transfer_buffer__extern(iree_hal_buffer_t *device_buffer) { return iree_hal_make_device_transfer_buffer(device_buffer); }
iree_hal_topology_edge_scheduling_word_t iree_hal_topology_edge_set_wait_mode__extern(iree_hal_topology_edge_scheduling_word_t word, iree_hal_topology_interop_mode_t mode) { return iree_hal_topology_edge_set_wait_mode(word, mode); }
iree_hal_topology_edge_scheduling_word_t iree_hal_topology_edge_set_signal_mode__extern(iree_hal_topology_edge_scheduling_word_t word, iree_hal_topology_interop_mode_t mode) { return iree_hal_topology_edge_set_signal_mode(word, mode); }
iree_hal_topology_edge_scheduling_word_t iree_hal_topology_edge_set_buffer_read_mode__extern(iree_hal_topology_edge_scheduling_word_t word, iree_hal_topology_interop_mode_t mode) { return iree_hal_topology_edge_set_buffer_read_mode(word, mode); }
iree_hal_topology_edge_scheduling_word_t iree_hal_topology_edge_set_buffer_write_mode__extern(iree_hal_topology_edge_scheduling_word_t word, iree_hal_topology_interop_mode_t mode) { return iree_hal_topology_edge_set_buffer_write_mode(word, mode); }
iree_hal_topology_edge_scheduling_word_t iree_hal_topology_edge_set_capability_flags__extern(iree_hal_topology_edge_scheduling_word_t word, iree_hal_topology_capability_t flags) { return iree_hal_topology_edge_set_capability_flags(word, flags); }
iree_hal_topology_edge_scheduling_word_t iree_hal_topology_edge_set_wait_cost__extern(iree_hal_topology_edge_scheduling_word_t word, uint8_t cost) { return iree_hal_topology_edge_set_wait_cost(word, cost); }
iree_hal_topology_edge_scheduling_word_t iree_hal_topology_edge_set_signal_cost__extern(iree_hal_topology_edge_scheduling_word_t word, uint8_t cost) { return iree_hal_topology_edge_set_signal_cost(word, cost); }
iree_hal_topology_edge_scheduling_word_t iree_hal_topology_edge_set_copy_cost__extern(iree_hal_topology_edge_scheduling_word_t word, uint8_t cost) { return iree_hal_topology_edge_set_copy_cost(word, cost); }
iree_hal_topology_edge_scheduling_word_t iree_hal_topology_edge_set_latency_class__extern(iree_hal_topology_edge_scheduling_word_t word, uint8_t latency_class) { return iree_hal_topology_edge_set_latency_class(word, latency_class); }
iree_hal_topology_edge_scheduling_word_t iree_hal_topology_edge_set_numa_distance__extern(iree_hal_topology_edge_scheduling_word_t word, uint8_t distance) { return iree_hal_topology_edge_set_numa_distance(word, distance); }
iree_hal_topology_edge_scheduling_word_t iree_hal_topology_edge_set_link_class__extern(iree_hal_topology_edge_scheduling_word_t word, iree_hal_topology_link_class_t link_class) { return iree_hal_topology_edge_set_link_class(word, link_class); }
iree_hal_topology_edge_interop_word_t iree_hal_topology_edge_set_semaphore_import_types__extern(iree_hal_topology_edge_interop_word_t word, iree_hal_topology_handle_type_t types) { return iree_hal_topology_edge_set_semaphore_import_types(word, types); }
iree_hal_topology_edge_interop_word_t iree_hal_topology_edge_set_semaphore_export_types__extern(iree_hal_topology_edge_interop_word_t word, iree_hal_topology_handle_type_t types) { return iree_hal_topology_edge_set_semaphore_export_types(word, types); }
iree_hal_topology_edge_interop_word_t iree_hal_topology_edge_set_buffer_import_types__extern(iree_hal_topology_edge_interop_word_t word, iree_hal_topology_handle_type_t types) { return iree_hal_topology_edge_set_buffer_import_types(word, types); }
iree_hal_topology_edge_interop_word_t iree_hal_topology_edge_set_buffer_export_types__extern(iree_hal_topology_edge_interop_word_t word, iree_hal_topology_handle_type_t types) { return iree_hal_topology_edge_set_buffer_export_types(word, types); }
iree_vm_ref_type_t iree_vm_make_ref_type__extern(const iree_vm_ref_type_descriptor_t *descriptor) { return iree_vm_make_ref_type(descriptor); }
const iree_vm_ref_type_descriptor_t * iree_vm_ref_type_descriptor__extern(iree_vm_ref_type_t type) { return iree_vm_ref_type_descriptor(type); }
iree_vm_ref_t iree_vm_ref_null__extern(void) { return iree_vm_ref_null(); }
iree_status_t iree_vm_ref_check__extern(const iree_vm_ref_t ref, iree_vm_ref_type_t type) { return iree_vm_ref_check(ref, type); }
iree_const_byte_span_t iree_vm_buffer_as_const_byte_span__extern(const iree_vm_buffer_t *value) { return iree_vm_buffer_as_const_byte_span(value); }
iree_string_view_t iree_vm_buffer_as_string__extern(const iree_vm_buffer_t *value) { return iree_vm_buffer_as_string(value); }
iree_vm_ref_type_t iree_vm_buffer_type__extern(void) { return iree_vm_buffer_type(); }
bool iree_vm_buffer_isa__extern(const iree_vm_ref_t ref) { return iree_vm_buffer_isa(ref); }
iree_vm_buffer_t * iree_vm_buffer_deref__extern(const iree_vm_ref_t ref) { return iree_vm_buffer_deref(ref); }
bool iree_vm_function_is_null__extern(iree_vm_function_t function) { return iree_vm_function_is_null(function); }
void * iree_vm_stack_frame_storage__extern(iree_vm_stack_frame_t *frame) { return iree_vm_stack_frame_storage(frame); }
iree_vm_value_t iree_vm_value_make_none__extern(void) { return iree_vm_value_make_none(); }
iree_vm_value_t iree_vm_value_make_i8__extern(int8_t value) { return iree_vm_value_make_i8(value); }
iree_vm_value_t iree_vm_value_make_i16__extern(int16_t value) { return iree_vm_value_make_i16(value); }
iree_vm_value_t iree_vm_value_make_i32__extern(int32_t value) { return iree_vm_value_make_i32(value); }
int32_t iree_vm_value_get_i32__extern(iree_vm_value_t *value) { return iree_vm_value_get_i32(value); }
iree_vm_value_t iree_vm_value_make_i64__extern(int64_t value) { return iree_vm_value_make_i64(value); }
int64_t iree_vm_value_get_i64__extern(iree_vm_value_t *value) { return iree_vm_value_get_i64(value); }
iree_vm_value_t iree_vm_value_make_f32__extern(float value) { return iree_vm_value_make_f32(value); }
float iree_vm_value_get_f32__extern(iree_vm_value_t *value) { return iree_vm_value_get_f32(value); }
iree_vm_value_t iree_vm_value_make_f64__extern(double value) { return iree_vm_value_make_f64(value); }
double iree_vm_value_get_f64__extern(iree_vm_value_t *value) { return iree_vm_value_get_f64(value); }
bool iree_vm_type_def_equal__extern(iree_vm_type_def_t a, iree_vm_type_def_t b) { return iree_vm_type_def_equal(a, b); }
iree_vm_type_def_t iree_vm_make_undefined_type_def__extern(void) { return iree_vm_make_undefined_type_def(); }
iree_vm_type_def_t iree_vm_make_value_type_def__extern(iree_vm_value_type_t value_type) { return iree_vm_make_value_type_def(value_type); }
iree_vm_type_def_t iree_vm_make_ref_type_def__extern(iree_vm_ref_type_t ref_type) { return iree_vm_make_ref_type_def(ref_type); }
iree_vm_variant_t iree_vm_variant_empty__extern(void) { return iree_vm_variant_empty(); }
bool iree_vm_variant_is_empty__extern(iree_vm_variant_t variant) { return iree_vm_variant_is_empty(variant); }
bool iree_vm_variant_is_value__extern(iree_vm_variant_t variant) { return iree_vm_variant_is_value(variant); }
bool iree_vm_variant_is_ref__extern(iree_vm_variant_t variant) { return iree_vm_variant_is_ref(variant); }
iree_vm_variant_t iree_vm_make_variant_value__extern(iree_vm_value_t value) { return iree_vm_make_variant_value(value); }
iree_vm_variant_t iree_vm_make_variant_ref_assign__extern(iree_vm_ref_t ref) { return iree_vm_make_variant_ref_assign(ref); }
iree_vm_value_t iree_vm_variant_value__extern(iree_vm_variant_t variant) { return iree_vm_variant_value(variant); }
void iree_vm_variant_reset__extern(iree_vm_variant_t *variant) { iree_vm_variant_reset(variant); }
iree_vm_ref_type_t iree_vm_list_type__extern(void) { return iree_vm_list_type(); }
bool iree_vm_list_isa__extern(const iree_vm_ref_t ref) { return iree_vm_list_isa(ref); }
iree_vm_list_t * iree_vm_list_deref__extern(const iree_vm_ref_t ref) { return iree_vm_list_deref(ref); }
iree_vm_abi_v_t * iree_vm_abi_v_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_v_checked_deref(buffer); }
void iree_vm_abi_v_reset__extern(iree_vm_abi_v_t *value) { iree_vm_abi_v_reset(value); }
iree_vm_abi_i_t * iree_vm_abi_i_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_i_checked_deref(buffer); }
void iree_vm_abi_i_reset__extern(iree_vm_abi_i_t *value) { iree_vm_abi_i_reset(value); }
iree_vm_abi_I_t * iree_vm_abi_I_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_I_checked_deref(buffer); }
void iree_vm_abi_I_reset__extern(iree_vm_abi_I_t *value) { iree_vm_abi_I_reset(value); }
iree_vm_abi_f_t * iree_vm_abi_f_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_f_checked_deref(buffer); }
void iree_vm_abi_f_reset__extern(iree_vm_abi_f_t *value) { iree_vm_abi_f_reset(value); }
iree_vm_abi_ii_t * iree_vm_abi_ii_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_ii_checked_deref(buffer); }
void iree_vm_abi_ii_reset__extern(iree_vm_abi_ii_t *value) { iree_vm_abi_ii_reset(value); }
iree_vm_abi_iI_t * iree_vm_abi_iI_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_iI_checked_deref(buffer); }
void iree_vm_abi_iI_reset__extern(iree_vm_abi_iI_t *value) { iree_vm_abi_iI_reset(value); }
iree_vm_abi_Ii_t * iree_vm_abi_Ii_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_Ii_checked_deref(buffer); }
void iree_vm_abi_Ii_reset__extern(iree_vm_abi_Ii_t *value) { iree_vm_abi_Ii_reset(value); }
iree_vm_abi_iIi_t * iree_vm_abi_iIi_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_iIi_checked_deref(buffer); }
void iree_vm_abi_iIi_reset__extern(iree_vm_abi_iIi_t *value) { iree_vm_abi_iIi_reset(value); }
iree_vm_abi_ir_t * iree_vm_abi_ir_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_ir_checked_deref(buffer); }
void iree_vm_abi_ir_reset__extern(iree_vm_abi_ir_t *value) { iree_vm_abi_ir_reset(value); }
iree_vm_abi_rir_t * iree_vm_abi_rir_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rir_checked_deref(buffer); }
void iree_vm_abi_rir_reset__extern(iree_vm_abi_rir_t *value) { iree_vm_abi_rir_reset(value); }
iree_vm_abi_iiir_t * iree_vm_abi_iiir_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_iiir_checked_deref(buffer); }
void iree_vm_abi_iiir_reset__extern(iree_vm_abi_iiir_t *value) { iree_vm_abi_iiir_reset(value); }
iree_vm_abi_II_t * iree_vm_abi_II_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_II_checked_deref(buffer); }
void iree_vm_abi_II_reset__extern(iree_vm_abi_II_t *value) { iree_vm_abi_II_reset(value); }
iree_vm_abi_IIi_t * iree_vm_abi_IIi_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_IIi_checked_deref(buffer); }
void iree_vm_abi_IIi_reset__extern(iree_vm_abi_IIi_t *value) { iree_vm_abi_IIi_reset(value); }
iree_vm_abi_iii_t * iree_vm_abi_iii_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_iii_checked_deref(buffer); }
void iree_vm_abi_iii_reset__extern(iree_vm_abi_iii_t *value) { iree_vm_abi_iii_reset(value); }
iree_vm_abi_iiii_t * iree_vm_abi_iiii_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_iiii_checked_deref(buffer); }
void iree_vm_abi_iiii_reset__extern(iree_vm_abi_iiii_t *value) { iree_vm_abi_iiii_reset(value); }
iree_vm_abi_irIi_t * iree_vm_abi_irIi_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_irIi_checked_deref(buffer); }
void iree_vm_abi_irIi_reset__extern(iree_vm_abi_irIi_t *value) { iree_vm_abi_irIi_reset(value); }
iree_vm_abi_irII_t * iree_vm_abi_irII_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_irII_checked_deref(buffer); }
void iree_vm_abi_irII_reset__extern(iree_vm_abi_irII_t *value) { iree_vm_abi_irII_reset(value); }
iree_vm_abi_r_t * iree_vm_abi_r_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_r_checked_deref(buffer); }
void iree_vm_abi_r_reset__extern(iree_vm_abi_r_t *value) { iree_vm_abi_r_reset(value); }
iree_vm_abi_rr_t * iree_vm_abi_rr_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rr_checked_deref(buffer); }
void iree_vm_abi_rr_reset__extern(iree_vm_abi_rr_t *value) { iree_vm_abi_rr_reset(value); }
iree_vm_abi_rrr_t * iree_vm_abi_rrr_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rrr_checked_deref(buffer); }
void iree_vm_abi_rrr_reset__extern(iree_vm_abi_rrr_t *value) { iree_vm_abi_rrr_reset(value); }
iree_vm_abi_ri_t * iree_vm_abi_ri_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_ri_checked_deref(buffer); }
void iree_vm_abi_ri_reset__extern(iree_vm_abi_ri_t *value) { iree_vm_abi_ri_reset(value); }
iree_vm_abi_rI_t * iree_vm_abi_rI_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rI_checked_deref(buffer); }
void iree_vm_abi_rI_reset__extern(iree_vm_abi_rI_t *value) { iree_vm_abi_rI_reset(value); }
iree_vm_abi_ririi_t * iree_vm_abi_ririi_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_ririi_checked_deref(buffer); }
void iree_vm_abi_ririi_reset__extern(iree_vm_abi_ririi_t *value) { iree_vm_abi_ririi_reset(value); }
iree_vm_abi_rii_t * iree_vm_abi_rii_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rii_checked_deref(buffer); }
void iree_vm_abi_rii_reset__extern(iree_vm_abi_rii_t *value) { iree_vm_abi_rii_reset(value); }
iree_vm_abi_rIi_t * iree_vm_abi_rIi_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIi_checked_deref(buffer); }
void iree_vm_abi_rIi_reset__extern(iree_vm_abi_rIi_t *value) { iree_vm_abi_rIi_reset(value); }
iree_vm_abi_rIIrrii_t * iree_vm_abi_rIIrrii_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIIrrii_checked_deref(buffer); }
void iree_vm_abi_rIIrrii_reset__extern(iree_vm_abi_rIIrrii_t *value) { iree_vm_abi_rIIrrii_reset(value); }
iree_vm_abi_rIirIIi_t * iree_vm_abi_rIirIIi_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIirIIi_checked_deref(buffer); }
void iree_vm_abi_rIirIIi_reset__extern(iree_vm_abi_rIirIIi_t *value) { iree_vm_abi_rIirIIi_reset(value); }
iree_vm_abi_rII_t * iree_vm_abi_rII_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rII_checked_deref(buffer); }
void iree_vm_abi_rII_reset__extern(iree_vm_abi_rII_t *value) { iree_vm_abi_rII_reset(value); }
iree_vm_abi_rif_t * iree_vm_abi_rif_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rif_checked_deref(buffer); }
void iree_vm_abi_rif_reset__extern(iree_vm_abi_rif_t *value) { iree_vm_abi_rif_reset(value); }
iree_vm_abi_riii_t * iree_vm_abi_riii_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_riii_checked_deref(buffer); }
void iree_vm_abi_riii_reset__extern(iree_vm_abi_riii_t *value) { iree_vm_abi_riii_reset(value); }
iree_vm_abi_riiii_t * iree_vm_abi_riiii_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_riiii_checked_deref(buffer); }
void iree_vm_abi_riiii_reset__extern(iree_vm_abi_riiii_t *value) { iree_vm_abi_riiii_reset(value); }
iree_vm_abi_riiIi_t * iree_vm_abi_riiIi_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_riiIi_checked_deref(buffer); }
void iree_vm_abi_riiIi_reset__extern(iree_vm_abi_riiIi_t *value) { iree_vm_abi_riiIi_reset(value); }
iree_vm_abi_riiI_t * iree_vm_abi_riiI_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_riiI_checked_deref(buffer); }
void iree_vm_abi_riiI_reset__extern(iree_vm_abi_riiI_t *value) { iree_vm_abi_riiI_reset(value); }
iree_vm_abi_iirII_t * iree_vm_abi_iirII_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_iirII_checked_deref(buffer); }
void iree_vm_abi_iirII_reset__extern(iree_vm_abi_iirII_t *value) { iree_vm_abi_iirII_reset(value); }
iree_vm_abi_rIiiI_t * iree_vm_abi_rIiiI_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIiiI_checked_deref(buffer); }
void iree_vm_abi_rIiiI_reset__extern(iree_vm_abi_rIiiI_t *value) { iree_vm_abi_rIiiI_reset(value); }
iree_vm_abi_riIiirII_t * iree_vm_abi_riIiirII_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_riIiirII_checked_deref(buffer); }
void iree_vm_abi_riIiirII_reset__extern(iree_vm_abi_riIiirII_t *value) { iree_vm_abi_riIiirII_reset(value); }
iree_vm_abi_rriiiirrIIIII_t * iree_vm_abi_rriiiirrIIIII_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rriiiirrIIIII_checked_deref(buffer); }
void iree_vm_abi_rriiiirrIIIII_reset__extern(iree_vm_abi_rriiiirrIIIII_t *value) { iree_vm_abi_rriiiirrIIIII_reset(value); }
iree_vm_abi_rriiiiI_t * iree_vm_abi_rriiiiI_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rriiiiI_checked_deref(buffer); }
void iree_vm_abi_rriiiiI_reset__extern(iree_vm_abi_rriiiiI_t *value) { iree_vm_abi_rriiiiI_reset(value); }
iree_vm_abi_rrIIiii_t * iree_vm_abi_rrIIiii_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rrIIiii_checked_deref(buffer); }
void iree_vm_abi_rrIIiii_reset__extern(iree_vm_abi_rrIIiii_t *value) { iree_vm_abi_rrIIiii_reset(value); }
iree_vm_abi_rrirI_t * iree_vm_abi_rrirI_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rrirI_checked_deref(buffer); }
void iree_vm_abi_rrirI_reset__extern(iree_vm_abi_rrirI_t *value) { iree_vm_abi_rrirI_reset(value); }
iree_vm_abi_rriirII_t * iree_vm_abi_rriirII_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rriirII_checked_deref(buffer); }
void iree_vm_abi_rriirII_reset__extern(iree_vm_abi_rriirII_t *value) { iree_vm_abi_rriirII_reset(value); }
iree_vm_abi_rrIrIIiI_t * iree_vm_abi_rrIrIIiI_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rrIrIIiI_checked_deref(buffer); }
void iree_vm_abi_rrIrIIiI_reset__extern(iree_vm_abi_rrIrIIiI_t *value) { iree_vm_abi_rrIrIIiI_reset(value); }
iree_vm_abi_riirIrIII_t * iree_vm_abi_riirIrIII_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_riirIrIII_checked_deref(buffer); }
void iree_vm_abi_riirIrIII_reset__extern(iree_vm_abi_riirIrIII_t *value) { iree_vm_abi_riirIrIII_reset(value); }
iree_vm_abi_rrIii_t * iree_vm_abi_rrIii_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rrIii_checked_deref(buffer); }
void iree_vm_abi_rrIii_reset__extern(iree_vm_abi_rrIii_t *value) { iree_vm_abi_rrIii_reset(value); }
iree_vm_abi_rrrIii_t * iree_vm_abi_rrrIii_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rrrIii_checked_deref(buffer); }
void iree_vm_abi_rrrIii_reset__extern(iree_vm_abi_rrrIii_t *value) { iree_vm_abi_rrrIii_reset(value); }
iree_vm_abi_rIrrIiiII_t * iree_vm_abi_rIrrIiiII_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIrrIiiII_checked_deref(buffer); }
void iree_vm_abi_rIrrIiiII_reset__extern(iree_vm_abi_rIrrIiiII_t *value) { iree_vm_abi_rIrrIiiII_reset(value); }
iree_vm_abi_rrIIIi_t * iree_vm_abi_rrIIIi_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rrIIIi_checked_deref(buffer); }
void iree_vm_abi_rrIIIi_reset__extern(iree_vm_abi_rrIIIi_t *value) { iree_vm_abi_rrIIIi_reset(value); }
iree_vm_abi_rrIIiIiI_t * iree_vm_abi_rrIIiIiI_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rrIIiIiI_checked_deref(buffer); }
void iree_vm_abi_rrIIiIiI_reset__extern(iree_vm_abi_rrIIiIiI_t *value) { iree_vm_abi_rrIIiIiI_reset(value); }
iree_vm_abi_rIrrrIIIiI_t * iree_vm_abi_rIrrrIIIiI_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIrrrIIIiI_checked_deref(buffer); }
void iree_vm_abi_rIrrrIIIiI_reset__extern(iree_vm_abi_rIrrrIIIiI_t *value) { iree_vm_abi_rIrrrIIIiI_reset(value); }
iree_vm_abi_rIrrrIrIII_t * iree_vm_abi_rIrrrIrIII_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIrrrIrIII_checked_deref(buffer); }
void iree_vm_abi_rIrrrIrIII_reset__extern(iree_vm_abi_rIrrrIrIII_t *value) { iree_vm_abi_rIrrrIrIII_reset(value); }
iree_vm_abi_rIrrrIrIIi_t * iree_vm_abi_rIrrrIrIIi_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIrrrIrIIi_checked_deref(buffer); }
void iree_vm_abi_rIrrrIrIIi_reset__extern(iree_vm_abi_rIrrrIrIIi_t *value) { iree_vm_abi_rIrrrIrIIi_reset(value); }
iree_vm_abi_rIrrrrrrr_t * iree_vm_abi_rIrrrrrrr_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIrrrrrrr_checked_deref(buffer); }
void iree_vm_abi_rIrrrrrrr_reset__extern(iree_vm_abi_rIrrrrrrr_t *value) { iree_vm_abi_rIrrrrrrr_reset(value); }
iree_vm_abi_rIrrrIiirrr_t * iree_vm_abi_rIrrrIiirrr_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIrrrIiirrr_checked_deref(buffer); }
void iree_vm_abi_rIrrrIiirrr_reset__extern(iree_vm_abi_rIrrrIiirrr_t *value) { iree_vm_abi_rIrrrIiirrr_reset(value); }
iree_vm_abi_rIrrI_t * iree_vm_abi_rIrrI_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIrrI_checked_deref(buffer); }
void iree_vm_abi_rIrrI_reset__extern(iree_vm_abi_rIrrI_t *value) { iree_vm_abi_rIrrI_reset(value); }
iree_vm_abi_rIrrr_t * iree_vm_abi_rIrrr_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIrrr_checked_deref(buffer); }
void iree_vm_abi_rIrrr_reset__extern(iree_vm_abi_rIrrr_t *value) { iree_vm_abi_rIrrr_reset(value); }
iree_vm_abi_rIrrrI_t * iree_vm_abi_rIrrrI_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIrrrI_checked_deref(buffer); }
void iree_vm_abi_rIrrrI_reset__extern(iree_vm_abi_rIrrrI_t *value) { iree_vm_abi_rIrrrI_reset(value); }
iree_vm_abi_rIrrCrD_t * iree_vm_abi_rIrrCrD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIrrCrD_checked_deref(buffer); }
iree_vm_abi_rIrrrICrIID_t * iree_vm_abi_rIrrrICrIID_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIrrrICrIID_checked_deref(buffer); }
iree_vm_abi_rCiD_t * iree_vm_abi_rCiD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rCiD_checked_deref(buffer); }
iree_vm_abi_rCrD_t * iree_vm_abi_rCrD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rCrD_checked_deref(buffer); }
iree_vm_abi_riCiD_t * iree_vm_abi_riCiD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_riCiD_checked_deref(buffer); }
iree_vm_abi_rIIiiCID_t * iree_vm_abi_rIIiiCID_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rIIiiCID_checked_deref(buffer); }
iree_vm_abi_rriiCID_t * iree_vm_abi_rriiCID_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rriiCID_checked_deref(buffer); }
iree_vm_abi_riCrD_t * iree_vm_abi_riCrD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_riCrD_checked_deref(buffer); }
iree_vm_abi_riiCriD_t * iree_vm_abi_riiCriD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_riiCriD_checked_deref(buffer); }
iree_vm_abi_rirCrD_t * iree_vm_abi_rirCrD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rirCrD_checked_deref(buffer); }
iree_vm_abi_rrrr_t * iree_vm_abi_rrrr_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rrrr_checked_deref(buffer); }
void iree_vm_abi_rrrr_reset__extern(iree_vm_abi_rrrr_t *value) { iree_vm_abi_rrrr_reset(value); }
iree_vm_abi_rrrrCrD_t * iree_vm_abi_rrrrCrD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rrrrCrD_checked_deref(buffer); }
iree_vm_abi_rriCiD_t * iree_vm_abi_rriCiD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rriCiD_checked_deref(buffer); }
iree_vm_abi_rrirCID_t * iree_vm_abi_rrirCID_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rrirCID_checked_deref(buffer); }
iree_vm_abi_riCiiiD_t * iree_vm_abi_riCiiiD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_riCiiiD_checked_deref(buffer); }
iree_vm_abi_rrCrIID_t * iree_vm_abi_rrCrIID_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rrCrIID_checked_deref(buffer); }
iree_vm_abi_rriCiirIID_t * iree_vm_abi_rriCiirIID_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_rriCiirIID_checked_deref(buffer); }
iree_vm_abi_CrD_t * iree_vm_abi_CrD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_CrD_checked_deref(buffer); }
iree_vm_abi_CrID_t * iree_vm_abi_CrID_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_CrID_checked_deref(buffer); }
iree_vm_abi_iiICrID_t * iree_vm_abi_iiICrID_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_iiICrID_checked_deref(buffer); }
iree_vm_abi_iICrD_t * iree_vm_abi_iICrD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_iICrD_checked_deref(buffer); }
iree_vm_abi_ICrD_t * iree_vm_abi_ICrD_checked_deref__extern(iree_byte_span_t buffer) { return iree_vm_abi_ICrD_checked_deref(buffer); }
iree_vm_ref_type_t iree_hal_allocator_type__extern(void) { return iree_hal_allocator_type(); }
bool iree_hal_allocator_isa__extern(const iree_vm_ref_t ref) { return iree_hal_allocator_isa(ref); }
iree_hal_allocator_t * iree_hal_allocator_deref__extern(const iree_vm_ref_t ref) { return iree_hal_allocator_deref(ref); }
iree_vm_ref_type_t iree_hal_buffer_type__extern(void) { return iree_hal_buffer_type(); }
bool iree_hal_buffer_isa__extern(const iree_vm_ref_t ref) { return iree_hal_buffer_isa(ref); }
iree_hal_buffer_t * iree_hal_buffer_deref__extern(const iree_vm_ref_t ref) { return iree_hal_buffer_deref(ref); }
iree_vm_ref_type_t iree_hal_buffer_view_type__extern(void) { return iree_hal_buffer_view_type(); }
bool iree_hal_buffer_view_isa__extern(const iree_vm_ref_t ref) { return iree_hal_buffer_view_isa(ref); }
iree_hal_buffer_view_t * iree_hal_buffer_view_deref__extern(const iree_vm_ref_t ref) { return iree_hal_buffer_view_deref(ref); }
iree_vm_ref_type_t iree_hal_channel_type__extern(void) { return iree_hal_channel_type(); }
bool iree_hal_channel_isa__extern(const iree_vm_ref_t ref) { return iree_hal_channel_isa(ref); }
iree_hal_channel_t * iree_hal_channel_deref__extern(const iree_vm_ref_t ref) { return iree_hal_channel_deref(ref); }
iree_vm_ref_type_t iree_hal_command_buffer_type__extern(void) { return iree_hal_command_buffer_type(); }
bool iree_hal_command_buffer_isa__extern(const iree_vm_ref_t ref) { return iree_hal_command_buffer_isa(ref); }
iree_hal_command_buffer_t * iree_hal_command_buffer_deref__extern(const iree_vm_ref_t ref) { return iree_hal_command_buffer_deref(ref); }
iree_vm_ref_type_t iree_hal_device_type__extern(void) { return iree_hal_device_type(); }
bool iree_hal_device_isa__extern(const iree_vm_ref_t ref) { return iree_hal_device_isa(ref); }
iree_hal_device_t * iree_hal_device_deref__extern(const iree_vm_ref_t ref) { return iree_hal_device_deref(ref); }
iree_vm_ref_type_t iree_hal_event_type__extern(void) { return iree_hal_event_type(); }
bool iree_hal_event_isa__extern(const iree_vm_ref_t ref) { return iree_hal_event_isa(ref); }
iree_hal_event_t * iree_hal_event_deref__extern(const iree_vm_ref_t ref) { return iree_hal_event_deref(ref); }
iree_vm_ref_type_t iree_hal_executable_type__extern(void) { return iree_hal_executable_type(); }
bool iree_hal_executable_isa__extern(const iree_vm_ref_t ref) { return iree_hal_executable_isa(ref); }
iree_hal_executable_t * iree_hal_executable_deref__extern(const iree_vm_ref_t ref) { return iree_hal_executable_deref(ref); }
iree_vm_ref_type_t iree_hal_executable_cache_type__extern(void) { return iree_hal_executable_cache_type(); }
bool iree_hal_executable_cache_isa__extern(const iree_vm_ref_t ref) { return iree_hal_executable_cache_isa(ref); }
iree_hal_executable_cache_t * iree_hal_executable_cache_deref__extern(const iree_vm_ref_t ref) { return iree_hal_executable_cache_deref(ref); }
iree_vm_ref_type_t iree_hal_fence_type__extern(void) { return iree_hal_fence_type(); }
bool iree_hal_fence_isa__extern(const iree_vm_ref_t ref) { return iree_hal_fence_isa(ref); }
iree_hal_fence_t * iree_hal_fence_deref__extern(const iree_vm_ref_t ref) { return iree_hal_fence_deref(ref); }
iree_vm_ref_type_t iree_hal_file_type__extern(void) { return iree_hal_file_type(); }
bool iree_hal_file_isa__extern(const iree_vm_ref_t ref) { return iree_hal_file_isa(ref); }
iree_hal_file_t * iree_hal_file_deref__extern(const iree_vm_ref_t ref) { return iree_hal_file_deref(ref); }
iree_vm_ref_type_t iree_hal_semaphore_type__extern(void) { return iree_hal_semaphore_type(); }
bool iree_hal_semaphore_isa__extern(const iree_vm_ref_t ref) { return iree_hal_semaphore_isa(ref); }
iree_hal_semaphore_t * iree_hal_semaphore_deref__extern(const iree_vm_ref_t ref) { return iree_hal_semaphore_deref(ref); }
iree_hal_executable_import_provider_t iree_hal_executable_import_provider_null__extern(void) { return iree_hal_executable_import_provider_null(); }
bool iree_hal_executable_import_is_optional__extern(const char *symbol_name) { return iree_hal_executable_import_is_optional(symbol_name); }
