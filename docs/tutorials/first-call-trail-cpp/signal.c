#include "signal.h"

static double clamp_sample(double sample) {
    if (sample > 1.0) {
        return 1.0;
    }
    if (sample < -1.0) {
        return -1.0;
    }
    return sample;
}

double prepare_sample(double sample) {
    return clamp_sample(sample);
}
