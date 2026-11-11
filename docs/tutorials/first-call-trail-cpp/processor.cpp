#include "signal.h"

static double apply_gain(double sample, double gain) {
    return sample * gain;
}

double process_sample(double sample) {
    const double limited = prepare_sample(sample);
    return apply_gain(limited, 0.5);
}
