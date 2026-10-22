#version 440

// One direction of a Gaussian blur, like CSS's blur(). Blurring horizontally, then vertically,
// gives the full blur. The result can be clipped to rounded corners.

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    // one texel along the direction of the blur, in texture coordinates
    vec2 texelStep;
    // the standard deviation, in texels
    float deviation;
    // the item's size and corner radius, in device pixels, to clip the result to
    vec2 size;
    float cornerRadius;
};

layout(binding = 1) uniform sampler2D source;

void main()
{
    vec4 sum = texture(source, qt_TexCoord0);
    if (deviation > 0.0) {
        // Out to three deviations, where the weights become negligible. Each sample is between two
        // texels, weighted so that bilinear filtering blends them in proportion, which halves the
        // samples needed.
        float radius = ceil(3.0 * deviation);
        float scale = -0.5 / (deviation * deviation);
        float total = 1.0;
        for (int i = 1; i <= 63; i += 2) {
            float near = float(i);
            if (near > radius)
                break;
            float nearWeight = exp(near * near * scale);
            float farWeight = exp((near + 1.0) * (near + 1.0) * scale);
            float weight = nearWeight + farWeight;
            vec2 offset = texelStep * ((near * nearWeight + (near + 1.0) * farWeight) / weight);
            sum += weight * (texture(source, qt_TexCoord0 + offset) + texture(source, qt_TexCoord0 - offset));
            total += 2.0 * weight;
        }
        sum /= total;
    }

    float coverage = 1.0;
    if (cornerRadius > 0.0) {
        vec2 p = qt_TexCoord0 * size;
        vec2 q = abs(p - 0.5 * size) - (0.5 * size - cornerRadius);
        float distance = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - cornerRadius;
        coverage = clamp(0.5 - distance, 0.0, 1.0);
    }
    fragColor = sum * coverage * qt_Opacity;
}
