# voxxelmaxx

A 3D falling sand game engine

## Design

- Written in Rust with Webgpu

- Chunks are 1x1x1 units, with variable voxels per chunk.

- Chunks live inside bodies. A body is a lattice of chunks that can move freely.

- Chunks have multiple buffers. Eg one buffer for the material tag, another buffer for water level.
  - > we can either have the buffers be type parameters, or use an entity component system to do "for each chunk with an X buffer do Y"

- Chunk -> render mesh and chunk -> collision mesh are overridable functions, but we provide sensible defaults and shared machinery such as triangle combining passes.

## Examples

Here is a list of use cases we would like to support

- Instead of cube voxels, rhombic dodecahedral (rad) voxels. There's a few ways to achieve this:
  - Use a skewed lattice, where the origin and the three axis generators make a regular simplex
  - Use every other voxel of the cubic grid. Ie you checker the cubic grid, then you cut each white cell into 6 pyramids and glue them on to the 6 neighboring black cells
    - This would require either leaving half the elements unused in the buffers, or having the dimensions of the buffers not be all the same length
  - have four rad voxel per cell of the cubic grid
If we do not want to compromise on bodies having orthonormal lattices and buffers being nxnxn, then we would have to use the third option.

- Instead of building on the cells of the grid, building on the faces.
  - Instead of a material tag per cell, we would need either:
    - Three buffers of material tags for the three orientations of faces
    - One buffer of structs holding three material tags each
   
- Maxwell's equations

- (Compressible) fluid simulation
  - > I'll have to look into whether we can get (compressible) navier-stokes to emerge from falling sand rules, or if there's a natural way to discretize the equations, or what
