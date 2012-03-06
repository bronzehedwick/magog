#include "transform.hpp"
#include "num.hpp"
#include <cmath>
#include <boost/assert.hpp>

// OpenGL Programming Guide, 7th Edition, page 807.
Gl_Matrix frustum(
    float l, float r,
    float b, float t,
    float n, float f) {
  BOOST_ASSERT(l != r);
  BOOST_ASSERT(b != t);
  BOOST_ASSERT(n != f);
  return Gl_Matrix {
    2*n / (r - l), 0,             (r + l) / (r - l),  0,
    0,             2*n / (t - b), (t + b) / (t - b),  0,
    0,             0,             -(f + n) / (f - n), -2*f*n / (f - n),
    0,             0,             -1,                 0};
}

// OpenGL Programming Guide, 7th Edition, page 808.
Gl_Matrix ortho(
    float l, float r,
    float b, float t,
    float n, float f) {
  BOOST_ASSERT(l != r);
  BOOST_ASSERT(b != t);
  BOOST_ASSERT(n != f);
  return Gl_Matrix {
    2 / (r - l), 0,           0,            -(r + l) / (r - l),
    0,           2 / (t - b), 0,            -(t + b) / (t - b),
    0,           0,           -2 / (f - n), (f + n) / (f - n),
    0,           0,           0,            0};
}

Gl_Matrix perspective(
    float v_fov, float aspect,
    float z_near, float z_far) {
  float fh = tan(v_fov / 360.0f * pi) * z_near;
  float fw = fh * aspect;
  return frustum(-fw, fw, -fh, fh, z_near, z_far);
}

// OpenGL Programming Guide, 7th Edition, page 806.
Gl_Matrix translation(const Vec3f& delta) {
  Gl_Matrix result;
  result.unit();
  for (int i = 0; i < 3; i++)
    result[3][i] = delta[i];
  return result;
}

// http://en.wikipedia.org/wiki/Rotation_matrix
Gl_Matrix rotation(const Vec3f& axis, float angle) {
  Vec3f u = axis;
  u.normalize();
  float x = u[0], y = u[1], z = u[2];
  float c = cos(angle), s = sin(angle);
  return Gl_Matrix{
    c + x*x*(1 - c),   x*y*(1 - c) - z*s, x*z*(1 - c) + y*s, 0,
    y*x*(1 - c) + z*s, c + y*y*(1 - c),   y*z*(1 - c) - x*s, 0,
    z*x*(1 - c) - y*s, z*y*(1 - c) + x*s, c + z*z*(1 - c),   0,
    0,                 0,                 0,                 1};
}

// http://www.j3d.org/matrix_faq/matrfaq_latest.html
Gl_Matrix rotation(const Quaternion& q) {
  return Gl_Matrix{
    1 - (2*q[2]*q[2] + 2*q[3]*q[3]), 2*q[1]*q[2] + 2*q[3]*q[0],       2*q[1]*q[3] - 2*q[2]*q[0],       0,
    2*q[1]*q[2] - 2*q[3]*q[0],       1 - (2*q[1]*q[1] + 2*q[3]*q[3]), 2*q[2]*q[3] + 2*q[1]*q[0],       0,
    2*q[1]*q[3] + 2*q[2]*q[0],       2*q[2]*q[3] - 2*q[1]*q[0],       1 - (2*q[1]*q[1] + 2*q[2]*q[2]), 0,
    0,                               0,                               0,                               1};
}
