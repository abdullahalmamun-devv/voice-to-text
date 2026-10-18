#!/usr/bin/env python3
"""Development-only, offline Whisper-small safetensors -> GGML F16 conversion.

Adapted mapping/layout from ggml-org/whisper.cpp v1.7.6 models/convert-h5-to-ggml.py
(MIT). No pickle, model Python code, Torch, network or training is used. Requires
numpy and safetensors. Local GGML template supplies verified mel filters/vocabulary;
all 50,257 base token IDs and architecture dimensions must match the fine-tune.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct

MAPPING = {
    'self_attn.k_proj':'attn.key', 'self_attn.q_proj':'attn.query',
    'self_attn.v_proj':'attn.value', 'self_attn.out_proj':'attn.out',
    'self_attn_layer_norm':'attn_ln', 'encoder_attn.q_proj':'cross_attn.query',
    'encoder_attn.k_proj':'cross_attn.key', 'encoder_attn.v_proj':'cross_attn.value',
    'encoder_attn.out_proj':'cross_attn.out', 'encoder_attn_layer_norm':'cross_attn_ln',
    'fc1':'mlp.0', 'fc2':'mlp.2', 'final_layer_norm':'mlp_ln',
    'encoder.layer_norm.bias':'encoder.ln_post.bias', 'encoder.layer_norm.weight':'encoder.ln_post.weight',
    'encoder.embed_positions.weight':'encoder.positional_embedding',
    'decoder.layer_norm.bias':'decoder.ln.bias', 'decoder.layer_norm.weight':'decoder.ln.weight',
    'decoder.embed_positions.weight':'decoder.positional_embedding',
    'decoder.embed_tokens.weight':'decoder.token_embedding.weight',
}


def tensor_name(source):
    parts = source.split('.')[1:]
    if len(parts) > 1 and parts[1] == 'layers':
        parts[1] = 'blocks'
        return '.'.join(parts[:3] + [MAPPING['.'.join(parts[3:-1])]] + parts[-1:])
    name = '.'.join(parts)
    return MAPPING.get(name, name)


def template_prefix(template, config, vocab):
    with template.open('rb') as stream:
        header = list(struct.unpack('<12i', stream.read(48)))
        expected = [config['vocab_size'], config['max_source_positions'], config['d_model'],
                    config['encoder_attention_heads'], config['encoder_layers'], config['max_target_positions'],
                    config['d_model'], config['decoder_attention_heads'], config['decoder_layers'], config['num_mel_bins']]
        if header[0] != 0x67676d6c or header[1:11] != expected:
            raise ValueError('Template architecture does not match')
        mel, fft = struct.unpack('<2i', stream.read(8))
        stream.seek(mel * fft * 4, 1)
        count = struct.unpack('<i', stream.read(4))[0]
        base = list(range(33, 127)) + list(range(161, 173)) + list(range(174, 256))
        extended = [value for value in range(256) if value not in base]
        byte_values = base + extended
        characters = base + list(range(256, 256 + len(extended)))
        decoder = {chr(character): value for value, character in zip(byte_values, characters)}
        tokens = {identifier: bytes(decoder[c] for c in token) for token, identifier in vocab.items()}
        for identifier in range(count):
            size = struct.unpack('<i', stream.read(4))[0]
            if stream.read(size) != tokens.get(identifier):
                raise ValueError('Tokenizer mismatch at ID ' + str(identifier))
        end = stream.tell()
        stream.seek(48)
        remainder = stream.read(end-48)
        header[-1] = 1
        return struct.pack('<12i', *header) + remainder


def main():
    import numpy as np
    from safetensors import safe_open
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('hf_folder', type=Path)
    parser.add_argument('template', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    config = json.loads((args.hf_folder/'config.json').read_text())
    vocab = json.loads((args.hf_folder/'vocab.json').read_text())
    prefix = template_prefix(args.template, config, vocab)
    names = set()
    with safe_open(args.hf_folder/'model.safetensors', framework='numpy') as weights, args.output.open('wb') as output:
        output.write(prefix)
        for source in sorted(weights.keys()):
            if source == 'proj_out.weight':
                continue
            name = tensor_name(source)
            if name in names:
                raise ValueError('Duplicate tensor mapping')
            names.add(name)
            data = weights.get_tensor(source).squeeze()
            if name in ['encoder.conv1.bias', 'encoder.conv2.bias']:
                data = data.reshape(data.shape[0], 1)
            f32 = data.ndim < 2 or name.endswith('positional_embedding') or name in ['encoder.conv1.bias', 'encoder.conv2.bias']
            data = np.asarray(data, dtype='<f4' if f32 else '<f2', order='C')
            encoded = name.encode()
            output.write(struct.pack('<3i', data.ndim, len(encoded), 0 if f32 else 1))
            output.write(struct.pack('<'+'i'*data.ndim, *reversed(data.shape)))
            output.write(encoded)
            data.tofile(output)
    with args.output.open('rb') as stream:
        sha = hashlib.file_digest(stream, 'sha256').hexdigest()
    print(json.dumps({'tensors':len(names), 'size_bytes':args.output.stat().st_size, 'sha256':sha}))


if __name__ == '__main__':
    main()
