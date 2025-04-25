#include <isa-l.h>

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "test.h"

using namespace std::chrono;

#define TEST_SOURCES 4
#define TEST_LEN WORKSET_SIZE
#define TEST_CNT(c) (TEST_LEN / c)
#define TEST_TYPE_STR ""

#define TEST_MEM ((TEST_SOURCES + 1) * (TEST_LEN))

int main(int argc, char *argv[])
{
  int i, ret, fail = 0;
  void **buffs;
  void *buff;
  // struct perf start;

  printf("Test xor_gen_perf\n");

  ret = posix_memalign((void **)&buff, 8, sizeof(int *) * (TEST_SOURCES + 6));
  if (ret)
  {
    printf("alloc error: Fail");
    return 1;
  }
  buffs = (void **)buff;

  // Allocate the arrays
  for (i = 0; i < TEST_SOURCES + 1; i++)
  {
    void *buf;
    ret = posix_memalign(&buf, 64, TEST_LEN);
    if (ret)
    {
      printf("alloc error: Fail");
      return 1;
    }
    buffs[i] = buf;
  }

  // Setup data
  for (i = 0; i < TEST_SOURCES + 1; i++)
    memset(buffs[i], 0, TEST_LEN);

  // Test latency
  for (int chunk_size = 64; chunk_size <= 16384; chunk_size *= 2)
  {
    int test_cnt = TEST_CNT(chunk_size);
    const int N = 1000000;
    uint8_t *srcs[TEST_SOURCES + 1];
    auto start = high_resolution_clock::now();
    for (int i = 0; i < N; ++i)
    {
      int t = i % test_cnt;
      for (int j = 0; j < TEST_SOURCES + 1; ++j)
      {
        srcs[j] = (uint8_t *)(buffs[j]) + t * chunk_size;
      }
      xor_gen_avx(TEST_SOURCES + 1, chunk_size, (void **)srcs);
    }
    auto end = high_resolution_clock::now();

    auto duration = duration_cast<nanoseconds>(end - start);
    printf("xor_gen_perf %5d: %f ns\n", chunk_size, (float)duration.count() / N);
  }

  return fail;
}
