#include "sherpa-onnx-c-api.h"
#include <stddef.h>
#include <stdio.h>
int main(void) {
  printf("%zu %zu %zu %zu %zu %zu %zu %zu %zu %zu %zu\n",
    sizeof(SherpaOnnxOnlineRecognizerConfig),
    sizeof(SherpaOnnxOnlineModelConfig),
    sizeof(SherpaOnnxOnlineCtcFstDecoderConfig),
    offsetof(SherpaOnnxOnlineRecognizerConfig, model_config),
    offsetof(SherpaOnnxOnlineRecognizerConfig, decoding_method),
    offsetof(SherpaOnnxOnlineRecognizerConfig, hotwords_file),
    offsetof(SherpaOnnxOnlineRecognizerConfig, ctc_fst_decoder_config),
    offsetof(SherpaOnnxOnlineRecognizerConfig, hotwords_buf),
    offsetof(SherpaOnnxOnlineRecognizerConfig, hr),
    offsetof(SherpaOnnxOnlineModelConfig, provider),
    offsetof(SherpaOnnxOnlineModelConfig, t_one_ctc));
  return 0;
}
