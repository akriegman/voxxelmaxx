# voxxelmaxx

A 3D falling sand game engine

## Design

- Written in Rust with Bevy

- Chunks are 1x1x1 units, with variable voxels per chunk.

- Chunks live inside bodies. A body is a lattice of chunks that moves as one rigid body.

- Chunks have multiple buffers. Eg one buffer for the material tag, another buffer for water level.
  - > we can either have the buffers be type parameters, or use an entity component system to do "for each chunk with an X buffer do Y"

- Chunk -> render mesh and chunk -> collision mesh are overridable functions, but we provide sensible defaults and shared machinery such as triangle combining passes.

### Physics

This might be more specific to the game I want to make with the engine. Idk I'll separate it out later. Working title "Phyzixx". Or "Phfthyiughszzuhihcktsxx". With thermodynamixx and electromaxxatism.

Every voxel holds a material and heat/internal energy. Fluid voxels hold a level.

Here's a game tick in pseudocode:
- calculate temperature
  - every material has an equation of state for temperature as a function of energy and level
- apply forces
  - find intersections of fluid voxels with solids. the volume of the fluid voxel is the volume outside any intersecting solid voxels.
  - each fluid has an equation of state that gives pressure as a function of level, volume, and temperature
  - each fluid voxel applies a force on each _face_ of a solid voxel intersecting it, equal to the pressure times the area of the intersection, normal to the face
  - each fluid voxel applies a force on each neighboring fluid voxel, again pressure times area but this time the surface is just a square of the grid.
    - > we could use the area of the face outside intersecting solids, or we could just treat these faces as all the same size.
  - gravity is applied to each fluid voxel
- move
  - solids move as rigid bodies, with angular momentum and all that
  - fluids:
    - move candidates = all neighbors in an orthant containing the force vector. ie in general the force will be in one orthant so the candidates will be one corner neighbor, three edge neighbors, and three face neighbors, but if the force is 0 then it lies in every orthant so the candidates are all the neighbors. but this is probably overcomplicated, and we should choose one orthant breaking ties arbitrarily.
    - if intersecting a solid voxel, cannot move into another voxel intersecting the same solid voxel.
    - if B is a candidate of A and A is a candidate of B, we can swap them
    - if a candidate has the same fluid type, we can transfer some level. how much should depend on the forces. maybe we can project the forces onto the displacement direction, then move a portion p of A to B, and a portion q of B to A, taking respective portions of the force with them, chosen to... minimize the remaining (squared) force? set the remaining force equal to 0? the equations become degenerate when the forces are equal... if we're minimizing then we can optionally skip the projection step I mentioned...
      - I guess that since we're neglecting momentum, minimizing force minimizes how incorrect our model is. This is what happens with Stokes flow, you neglect the momentum terms and you're just left with F = 0.
    - I'm not sure how we should prioritize those moves. I guess that the order we consider the candidates should depend on the forces.
- conduct heat
- I guess that after things move we then have to withdraw energy from fluid voxels as (force applied to solid) dot (distance that solid voxel moved)

And we'll also have an EM tick, which mostly doesn't interact with the above. We also may want to consider rewriting the above to use a momentum field. It would allow for more realistic fluid sim, but probably a lot more compute for a little more fun.

Some materials:
- iron has an intrinsic magnetic field that can rotate between six directions based on the temperature, so we get a voxel level Ising model sorta
- silicon, doped silicon
- lithium (idk how batteries work)
- glass. mirror?
- steam/water vapor (same material at different temperatures)
- fabric / rope / chains holds a list of linked voxels (displacements not pointers), and can only move in ways that maintain adjacency.
  - this might be the wrong idea but we can bit pack: each fabric voxel has four neighbors, one byte per neighbor. 2 bits for the x displacement, 00 means 0, 01 means 1, 10 means -1. 2 bits for y, 2 for z. Then 2 bits for the index of this voxel in that neighbor's array of neighbors. And the null byte indicates a missing neighbor.
  - or alternatively we just add thread and you have to make fabric by weaving
  - to find the allowed moves, you | the neighbor bytes together. For the x bits, the first bit then indicates whether you can increment x, and the second indicates whether you can decrement x.

No inventory. You can hold one item. If you hold left click, then moving the mouse will rotate the object around the voxel you're holding it from, instead of turning the camera. So you would fight and mine by actually swinging your weapon/tool. Hold shift to go into third person mode and maniupulate things with your offhand. This should allow you to put on a backpack, reload a gun, etc

There are some special interactions that we will need "items" for. Maybe a hammer item.
- smash a body into individual voxels so that they can settle into the world grid, deallocating. there are other ways we can do this...
- raise the resolution of a body
- if we want chains / chainmail voxels, we can make them using the hammer. alternatively players can make actual chains...

## Examples

I would also like to support some weird geometry.

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
