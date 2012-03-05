#ifndef UTIL_CORE_HPP
#define UTIL_CORE_HPP

#include <cstddef>

template<size_t N, size_t I=0>
struct _hash_calc {
  static constexpr size_t apply(const char (&s)[N]) {
    return (_hash_calc<N, I+1>::apply(s) ^ s[I]) * 16777619u;
  }
};

template<size_t N>
struct _hash_calc<N, N> {
  static constexpr size_t apply(const char (&s)[N]) {
    return 2166136261u;
  }
};

template<size_t N>
constexpr size_t const_hash(const char (&s)[N]) {
  return _hash_calc<N>::apply(s);
}

size_t hash(const char* s);

void die(const char* format, ...);

#ifdef NDEBUG
#define ASSERT(expr) ((void)0)
#else
#define ASSERT(expr) ((expr) ? ((void)0) : die("Assertion %s failed at %s: %d", #expr, __FILE__, __LINE__))
#endif

#endif