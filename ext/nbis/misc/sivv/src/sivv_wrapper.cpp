#include <opencv2/core.hpp>
#include <opencv2/imgproc.hpp>
#include <cstring>
#include <string>
#include "fingerprint_wrapper.h"
#include "SIVVCore.h"

extern std::string sivv(const cv::Mat &src);

extern "C" {

char* sivv_ffi_from_bytes(const unsigned char* data, int width, int height) {
    if (!data || width <= 0 || height <= 0) {
        return nullptr;
    }
    cv::Mat view(height, width, CV_8UC1, const_cast<unsigned char*>(data));
    cv::Mat img = view.clone();
    std::string result = sivv(img);
    char* out = static_cast<char*>(std::malloc(result.size() + 1));
    if (!out) {
        return nullptr;
    }
    std::memcpy(out, result.c_str(), result.size() + 1);
    return out;
}

void sivv_ffi_free_bytes(char* ptr) {
    std::free(ptr);
}

CPoint2i find_fingerprint_center_morph_c(
        const unsigned char* data,
        int width,
        int height,
        int* xbound_min,
        int* xbound_max,
        int* ybound_min,
        int* ybound_max)
{
    CPoint2i c_result{};
    if (!data || width <= 0 || height <= 0
        || !xbound_min || !xbound_max || !ybound_min || !ybound_max) {
        return c_result;
    }

    cv::Mat view(height, width, CV_8UC1, const_cast<unsigned char*>(data));
    cv::Mat img = view.clone();
    cv::Point2i result = find_fingerprint_center_morph(
            img, xbound_min, xbound_max, ybound_min, ybound_max);

    c_result.x = result.x;
    c_result.y = result.y;
    return c_result;
}

} // extern "C"
