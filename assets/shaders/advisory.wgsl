struct Features {
    values: array<f32, 84>,
}

struct Weights {
    values: array<f32, 57>,
}

struct Scores {
    values: array<f32, 7>,
}

@group(0) @binding(0) var<storage, read> features: Features;
@group(0) @binding(1) var<storage, read> weights: Weights;
@group(0) @binding(2) var<storage, read_write> scores: Scores;

@compute @workgroup_size(8)
fn score_world(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let world_index = invocation.x;
    if (world_index >= 7u) {
        return;
    }

    var hidden: array<f32, 4>;
    for (var hidden_index = 0u; hidden_index < 4u; hidden_index += 1u) {
        var activation = weights.values[48u + hidden_index];
        for (var feature_index = 0u; feature_index < 12u; feature_index += 1u) {
            let feature = features.values[world_index * 12u + feature_index];
            activation += weights.values[hidden_index * 12u + feature_index] * feature;
        }
        hidden[hidden_index] = max(activation, 0.0);
    }

    var score = weights.values[56u];
    for (var hidden_index = 0u; hidden_index < 4u; hidden_index += 1u) {
        score += hidden[hidden_index] * weights.values[52u + hidden_index];
    }
    scores.values[world_index] = clamp(score, 0.0, 1.0);
}
