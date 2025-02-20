/** zn and zf are the (both negative) near and far planes
    used for the projection matrix that renders the scene.

    From the Graphics Codex entry for "Unproject Depth".
*/ 
vec3 computeClipInfo(float zn, float zf) { 
    if (zf == -INF) {
        return vec3(zn, -1.0f, +1.0f);
    } else {
        return vec3(zn  * zf, zn - zf, zf);
    }
}

