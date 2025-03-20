# Radiance Cascades in 2D and 3D using OpenGL
A 2D and a 3D implementation of [Radiance Cascades](https://github.com/Raikiri/RadianceCascadesPaper) by Alexander Sannikov. 

## Framework
The implementations are done in Rust using [glow](https://github.com/grovesNL/glow) for OpenGL bindings. The [MicroGLUT port to Rust](https://gitlab.liu.se/gusso230/microglut) by my friend Gustav Sörnäs, based on [MicroGLUT](https://computer-graphics.se/packages/microglut.html) by Ingemar Ragnemalm, is used for is used as a small wrapper for windown and event handling. 

## 2D implementation
The shaders are pretty much ports of the [2D implementation by Yaazarai](https://github.com/Yaazarai/RadianceCascades). Definitely check out Yaazarai's implementation!

## 3D implementation (SPWI)
Probes are placed in screen space and projected onto the depth buffer. Radiance intervals are traced in world space using a voxelisation of the scene.

## Resources and useful links
- The [paper](https://github.com/Raikiri/RadianceCascadesPaper) by Alexander Sannikov
- The awesome community at the [Radiance Cascade discord server](https://discord.gg/USwhaBXuSF)
- https://radiance-cascades.com/
- https://tmpvar.com/poc/radiance-cascades/
