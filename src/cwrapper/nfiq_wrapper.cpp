// nfiq_wrapper.cpp
#include "nfiq_wrapper.h"
#include <nfiq2.hpp>
#include <cstdlib>
#include <cstring>

struct Nfiq2Wrapper {
    NFIQ2::Algorithm model;
};

namespace {

void free_string_array(const char** ids, uint32_t count) {
    if (!ids) {
        return;
    }
    for (uint32_t i = 0; i < count; ++i) {
        std::free(const_cast<char*>(ids[i]));
    }
    std::free(const_cast<char**>(ids));
}

} // namespace

extern "C" {

Nfiq2Wrapper* nfiq2wrapper_create() {
    try {
        return new Nfiq2Wrapper{};
    } catch (...) {
        return nullptr;
    }
}

void nfiq2wrapper_destroy(Nfiq2Wrapper* ctx) {
    if (ctx) {
        delete ctx;
    }
}

int nfiq2wrapper_compute(Nfiq2Wrapper*    ctx,
                         const uint8_t*   data,
                         uint32_t         size,
                         uint32_t         cols,
                         uint32_t         rows,
                         uint16_t         ppi,
                         nfiq2_results_t* out)
{
    if (!ctx || !data || !out || size != cols * rows || cols == 0 || rows == 0) {
        return 1;
    }
    // FingerJet quality measures are unreliable / crash-prone on tiny images.
    if (cols < 96 || rows < 96) {
        return 3;
    }

    std::memset(out, 0, sizeof(*out));

    try {
        NFIQ2::FingerprintImageData img(data, size, cols, rows, 0 /*dpi units*/, ppi);

        auto algos = NFIQ2::QualityMeasures::computeNativeQualityMeasureAlgorithms(img);

        out->score = ctx->model.computeUnifiedQualityScore(img);

        auto act_ids = NFIQ2::QualityMeasures::getActionableQualityFeedbackIDs();
        auto act_map = NFIQ2::QualityMeasures::getActionableQualityFeedback(algos);

        const auto act_n = static_cast<uint32_t>(act_ids.size());
        out->actionable_count = act_n;
        if (act_n > 0) {
            out->actionable_ids =
                static_cast<const char**>(std::malloc(sizeof(char*) * act_n));
            out->actionable_values =
                static_cast<double*>(std::malloc(sizeof(double) * act_n));
            if (!out->actionable_ids || !out->actionable_values) {
                nfiq2wrapper_free_results(out);
                return 2;
            }
            for (uint32_t i = 0; i < act_n; ++i) {
                const auto& id = act_ids[i];
                char* copy = static_cast<char*>(std::malloc(id.size() + 1));
                if (!copy) {
                    nfiq2wrapper_free_results(out);
                    return 2;
                }
                std::memcpy(copy, id.c_str(), id.size() + 1);
                out->actionable_ids[i] = copy;
                out->actionable_values[i] = act_map.at(id);
            }
        }

        auto feat_ids = NFIQ2::QualityMeasures::getNativeQualityMeasureIDs();
        auto feat_map = NFIQ2::QualityMeasures::getNativeQualityMeasures(algos);

        const auto feat_n = static_cast<uint32_t>(feat_ids.size());
        out->feature_count = feat_n;
        if (feat_n > 0) {
            out->feature_ids =
                static_cast<const char**>(std::malloc(sizeof(char*) * feat_n));
            out->feature_values =
                static_cast<double*>(std::malloc(sizeof(double) * feat_n));
            if (!out->feature_ids || !out->feature_values) {
                nfiq2wrapper_free_results(out);
                return 2;
            }
            for (uint32_t i = 0; i < feat_n; ++i) {
                const auto& id = feat_ids[i];
                char* copy = static_cast<char*>(std::malloc(id.size() + 1));
                if (!copy) {
                    nfiq2wrapper_free_results(out);
                    return 2;
                }
                std::memcpy(copy, id.c_str(), id.size() + 1);
                out->feature_ids[i] = copy;
                out->feature_values[i] = feat_map.at(id);
            }
        }

        return 0;
    } catch (...) {
        nfiq2wrapper_free_results(out);
        return 2;
    }
}

void nfiq2wrapper_free_results(nfiq2_results_t* out) {
    if (!out) {
        return;
    }

    free_string_array(out->actionable_ids, out->actionable_count);
    std::free(out->actionable_values);

    free_string_array(out->feature_ids, out->feature_count);
    std::free(out->feature_values);

    std::memset(out, 0, sizeof(*out));
}

} // extern "C"
