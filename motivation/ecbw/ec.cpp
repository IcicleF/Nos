#include <isa-l.h>

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "test.h"

#define TEST_SOURCES 32
#define TEST_LEN(m) WORKSET_SIZE
#define TEST_CNT(m, c) (TEST_LEN(m) / c)
#define TEST_TYPE_STR ""

#define MMAX TEST_SOURCES
#define KMAX TEST_SOURCES

#define BAD_MATRIX -1

typedef unsigned char u8;

using namespace std::chrono;

void ec_encode_perf(int m, int k, u8 *a, u8 *g_tbls, u8 **buffs)
{
  ec_init_tables(k, m - k, &a[k * k], g_tbls);
  for (int chunk_size = 64; chunk_size <= 16384; chunk_size *= 2)
  {
    int test_cnt = TEST_CNT(m, chunk_size);

    // Test latency
    const int N = 1000000;
    uint8_t *srcs[m];
    auto start = high_resolution_clock::now();
    for (int i = 0; i < N; ++i)
    {
      int t = i % test_cnt;
      for (int j = 0; j < m; ++j)
      {
        srcs[j] = (uint8_t *)(buffs[j]) + t * chunk_size;
      }
      ec_encode_data_avx(chunk_size, k, m - k, g_tbls, srcs, &srcs[k]);
    }
    auto end = high_resolution_clock::now();

    auto duration = duration_cast<nanoseconds>(end - start);
    printf("ec_encode_data_perf %5d: %f ns\n", chunk_size,
           (float)duration.count() / N);
  }
}

int main(int argc, char *argv[])
{
  int i, j, m, k, nerrs, check;
  void *buf;
  u8 *temp_buffs[TEST_SOURCES], *buffs[TEST_SOURCES];
  u8 a[MMAX * KMAX];
  u8 g_tbls[KMAX * TEST_SOURCES * 32], src_in_err[TEST_SOURCES];
  u8 src_err_list[TEST_SOURCES];
  // struct perf start;

  // Pick test parameters
  m = 5;
  k = 4;
  nerrs = 1;
  const u8 err_list[] = {2};

  printf("erasure_code_perf: %dx%d %d\n", m, TEST_LEN(m), nerrs);

  if (m > MMAX || k > KMAX || nerrs > (m - k))
  {
    printf(" Input test parameter error\n");
    return -1;
  }

  memcpy(src_err_list, err_list, nerrs);
  memset(src_in_err, 0, TEST_SOURCES);
  for (i = 0; i < nerrs; i++)
    src_in_err[src_err_list[i]] = 1;

  // Allocate the arrays
  for (i = 0; i < m; i++)
  {
    if (posix_memalign(&buf, 64, TEST_LEN(m)))
    {
      printf("alloc error: Fail\n");
      return -1;
    }
    buffs[i] = (u8 *)buf;
  }

  for (i = 0; i < (m - k); i++)
  {
    if (posix_memalign(&buf, 64, TEST_LEN(m)))
    {
      printf("alloc error: Fail\n");
      return -1;
    }
    temp_buffs[i] = (u8 *)buf;
  }

  // Make random data
  for (i = 0; i < k; i++)
    for (j = 0; j < TEST_LEN(m); j++)
      buffs[i][j] = rand();

  gf_gen_rs_matrix(a, m, k);

  // Start encode test
  ec_encode_perf(m, k, a, g_tbls, buffs);
  return 0;
}
