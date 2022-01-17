glslc.exe shader/full_screen_triangle.vert -o shader/full_screen_triangle_vert.spv

glslc.exe shader/main.vert -o shader/main_vert.spv
glslc.exe shader/main.frag -o shader/main_frag.spv
glslc.exe shader/main_frontface.vert -o shader/main_frontface_vert.spv
glslc.exe shader/simple.frag -o shader/simple_frag.spv

glslc.exe shader/vbuffer.vert -o shader/vbuffer_vert.spv
glslc.exe shader/vbuffer.frag -o shader/vbuffer_frag.spv

glslc.exe shader/depth_pyramid_first_mip.comp -o shader/depth_pyramid_first_mip.spv
glslc.exe shader/depth_pyramid_downsample.comp -o shader/depth_pyramid_downsample.spv
glslc.exe shader/depth_pyramid_downsample_all.comp -o shader/depth_pyramid_downsample_all.spv

glslc.exe shader/culling.comp -o shader/culling.spv
glslc.exe shader/culling_debug.frag -o shader/culling_debug_frag.spv

