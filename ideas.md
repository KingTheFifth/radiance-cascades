## RC
- Octahedral distribution or [theta, cos phi] for ray directions
- SH = Spherical harmonics
- Lowes level quadrature, 1 ray/octant

About multi-bounce:
- https://discord.com/channels/1277053173764460625/1277053574425350224/1322445910294138922

C-1 gathering:
- https://www.shadertoy.com/view/4ctXD8
- https://discord.com/channels/1277053173764460625/1277053175928590440/1302994378373070870
- https://discord.com/channels/1277053173764460625/1277053574425350224/1298168188302786590

Reflections and specular:
- https://discord.com/channels/1277053173764460625/1277053175928590440/1294359025231597709

SPWI omptimisations:
- https://discord.com/channels/1277053173764460625/1277053673549070346/1280704769308102796
- use f16 instead of f32 (half-precision) where possible
- trace min & max probe at the same time (https://discord.com/channels/1277053173764460625/1293089702315950080/1293409822095708171)
- prefilter depth & colour buffers => stability (https://discord.com/channels/1277053173764460625/1277053673549070346/1280712298113138698)
    - on filtering the depth buffer https://discord.com/channels/1277053173764460625/1277053673549070346/1281244492296355884

### Alternatives for ray casting
- SDF + ray marching
    - Library for SDF generation
    - E.g. brixelizer (open source MIT-licence) which handles SDF + ray marching
- Voxelisation + ray marching (?)
    - Might save me some work for VXGI
    - Probably not too hard to implement
- Ray tracing with BVHs
    - Library for building BVHs
        - Radeon Rays
    - Scary! Will it run in real-time?
    - Might enable things like refraction?
    - Might also straight up enable path tracing to compare with rather than VXGI (or all 3 techniques!)
- Screen-space ray marching
    - No contribution from off-screen (probably fine)
        - Would perhaps make it harder to compare with VXGI
    - Should run in real-time

### Random showcases:
    - https://discord.com/channels/1277053173764460625/1277060980215517195/1281365514467147838
    - https://discord.com/channels/1277053173764460625/1277060812913115269/1277617378456375434
    - https://discord.com/channels/1277053173764460625/1277060812913115269/1279768991677681746

## VXGI
- Use NVIDIA VXGI SDK + bindings
- Generating bindings
    - SWIG (www.swig.org)
    - CppSharp (GitHub.com/mono/CppSharp)
