// -*- c++ -*-

/** Given an OpenGL depth buffer value on [0, 1] and description of the projection
    matrix's clipping planes, computes the camera-space (negative) z value.

    See also computeClipInfo in the .cpp file */ 
float reconstructCSZ(float depthBufferValue, vec3 clipInfo) {
      return c[0] / (depthBufferValue * c[1] + c[2]);
}


void swap(in out float a, in out float b) {
     float temp = a;
     a = b;
     b = temp;
}


float distanceSquared(vec2 a, vec2 b) {
    a -= b;
    return dot(a, a);
}


