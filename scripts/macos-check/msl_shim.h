#include <cmath>
typedef unsigned int uint;
#define constant const
#define fragment
#define vertex
#define constexpr_sampler
template<int N> struct boolv { bool v[N]; };
struct float2 { float x, y;
  float2() : x(0), y(0) {} explicit float2(float s) : x(s), y(s) {} float2(float a, float b) : x(a), y(b) {}
  float operator[](int i) const { return i ? y : x; } };
struct float3 { float x, y, z;
  float3() : x(0), y(0), z(0) {} explicit float3(float s) : x(s), y(s), z(s) {} float3(float a, float b, float c) : x(a), y(b), z(c) {} };
struct float4 { float x, y, z, w;
  float4() : x(0), y(0), z(0), w(0) {} explicit float4(float s) : x(s), y(s), z(s), w(s) {}
  float4(float a, float b, float c, float d) : x(a), y(b), z(c), w(d) {}
  float4(float2 a, float2 b) : x(a.x), y(a.y), z(b.x), w(b.y) {}
  float4(float3 a, float b) : x(a.x), y(a.y), z(a.z), w(b) {}
  float4(float2 a, float b, float c) : x(a.x), y(a.y), z(b), w(c) {}
  float2 xy() const { return float2(x, y); } float2 zw() const { return float2(z, w); }
  float2 yz() const { return float2(y, z); } float3 xyz() const { return float3(x, y, z); } };
#define V2(OP) inline float2 operator OP(float2 a, float2 b){return float2(a.x OP b.x, a.y OP b.y);} \
 inline float2 operator OP(float2 a, float b){return float2(a.x OP b, a.y OP b);} \
 inline float2 operator OP(float a, float2 b){return float2(a OP b.x, a OP b.y);}
#define V3(OP) inline float3 operator OP(float3 a, float3 b){return float3(a.x OP b.x, a.y OP b.y, a.z OP b.z);} \
 inline float3 operator OP(float3 a, float b){return float3(a.x OP b, a.y OP b, a.z OP b);} \
 inline float3 operator OP(float a, float3 b){return float3(a OP b.x, a OP b.y, a OP b.z);}
#define V4(OP) inline float4 operator OP(float4 a, float4 b){return float4(a.x OP b.x, a.y OP b.y, a.z OP b.z, a.w OP b.w);} \
 inline float4 operator OP(float4 a, float b){return float4(a.x OP b, a.y OP b, a.z OP b, a.w OP b);} \
 inline float4 operator OP(float a, float4 b){return float4(a OP b.x, a OP b.y, a OP b.z, a OP b.w);}
V2(+) V2(-) V2(*) V2(/) V3(+) V3(-) V3(*) V3(/) V4(+) V4(-) V4(*) V4(/)
inline float2 operator-(float2 a){return float2(-a.x,-a.y);} inline float3 operator-(float3 a){return float3(-a.x,-a.y,-a.z);}
inline float2& operator*=(float2& a, float b){a = a*b; return a;}
inline float3& operator*=(float3& a, float b){a = a*b; return a;}
inline float4& operator*=(float4& a, float b){a = a*b; return a;}
inline boolv<2> operator>=(float2 a, float2 b){return {{a.x>=b.x, a.y>=b.y}};}
inline boolv<3> operator<=(float3 a, float3 b){return {{a.x<=b.x, a.y<=b.y, a.z<=b.z}};}
inline float2 select(float2 a, float2 b, boolv<2> c){return float2(c.v[0]?b.x:a.x, c.v[1]?b.y:a.y);}
inline float3 select(float3 a, float3 b, boolv<3> c){return float3(c.v[0]?b.x:a.x, c.v[1]?b.y:a.y, c.v[2]?b.z:a.z);}
inline float dot(float2 a, float2 b){return a.x*b.x+a.y*b.y;} inline float dot(float3 a, float3 b){return a.x*b.x+a.y*b.y+a.z*b.z;}
inline float length(float2 a){return std::sqrt(dot(a,a));}
inline float2 normalize(float2 a){return a/length(a);}
inline float max(float a, float b){return a>b?a:b;} inline float min(float a, float b){return a<b?a:b;}
inline float2 max(float2 a, float2 b){return float2(max(a.x,b.x),max(a.y,b.y));}
inline float3 max(float3 a, float3 b){return float3(max(a.x,b.x),max(a.y,b.y),max(a.z,b.z));}
inline float4 max(float4 a, float4 b){return float4(max(a.x,b.x),max(a.y,b.y),max(a.z,b.z),max(a.w,b.w));}
inline float2 abs(float2 a){return float2(std::fabs(a.x),std::fabs(a.y));}
inline float clamp(float x, float a, float b){return min(max(x,a),b);}
inline float saturate(float x){return clamp(x,0.f,1.f);}
inline float3 saturate(float3 x){return float3(saturate(x.x),saturate(x.y),saturate(x.z));}
inline float3 mix(float3 a, float3 b, float t){return a+(b-a)*t;}
inline float smoothstep(float a, float b, float x){float t=saturate((x-a)/(b-a)); return t*t*(3-2*t);}
inline float exp(float x){return std::exp(x);}
inline float3 sqrt(float3 a){return float3(std::sqrt(a.x),std::sqrt(a.y),std::sqrt(a.z));}
template<typename T> struct texture2d {};
struct Corners_ScaledPixels {};
float gaussian(float x, float sigma);
float blur_along_x(float x, float y, float sigma, float corner, float2 half_size);
