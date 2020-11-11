# Rust & Vulkan test projects
Contains test projects for Rust & Vulkan

The first test project renders 1 million cubes, each containing a 950 MB (uncompressed) distance field volume. It uses an optimized cube renderer rendering only front faces of each cube. 

The second test project is going to be using sparse octree storing a hierarchy of distance field volume bricks, each rasterized as a cube. This will both reduce the SDF volume memory consumption by 98% and make the runtime faster, as most rays missing the object will not be cast at at all, and the remaining rays will start very close to the surface.

The second test project will use a GPU-driven culling solution slightly similar to the one we presented at SIGGRAPH 2015:
https://advances.realtimerendering.com/s2015/aaltonenhaar_siggraph2015_combined_final_footer_220dpi.pdf

Various optimization techniques will be tested on top of this prototype.

# Instructions
* Install git LFS: https://git-lfs.github.com/
* Run (cmd): **git lfs install**
* Clone repository. Zip download does NOT support git LFS!
* Install rustup: https://www.rust-lang.org/tools/install
* Run (cmd): **cargo run --release**

## License
This repository contents are released under the MIT license. See [LICENSE.md](LICENSE.md) for full text.
