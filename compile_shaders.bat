glslc.exe shader/main.vert -o shader/main_vert.spv
glslc.exe shader/main.frag -o shader/main_frag.spv
glslc.exe shader/main_frontface.vert -o shader/main_frontface_vert.spv

glslc.exe shader/simple.frag -o shader/simple_frag.spv

glslc.exe shader/depth_pyramid_first_mip.comp -o shader/depth_pyramid_first_mip.spv
glslc.exe shader/depth_pyramid_downsample.comp -o shader/depth_pyramid_downsample.spv
glslc.exe shader/depth_pyramid_downsample_all.comp -o shader/depth_pyramid_downsample_all.spv
