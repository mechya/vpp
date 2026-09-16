// SVG path filling: parse, flatten, scanline-rasterize with coverage.

#include "vpp/svg.h"

#include <algorithm>
#include <cctype>
#include <cmath>
#include <cstdlib>
#include <vector>

namespace vpp {

namespace {

constexpr float kPi = 3.14159265358979f;

struct Point {
    float x, y;
};

using Polygon = std::vector<Point>;

class PathParser {
public:
    PathParser(const std::string& d, const PathTransform& t) : d_(d), t_(t) {}

    std::vector<Polygon> run() {
        while (skipSeparators(), i_ < d_.size()) {
            const char c = d_[i_];
            if (std::isalpha(static_cast<unsigned char>(c))) {
                cmd_ = c;
                ++i_;
            } else if (cmd_ == 0) {
                break;
            } else if (cmd_ == 'M') {
                cmd_ = 'L'; // implicit lineto after moveto
            } else if (cmd_ == 'm') {
                cmd_ = 'l';
            }
            if (!execute()) break;
        }
        close();
        return std::move(polys_);
    }

private:
    void skipSeparators() {
        while (i_ < d_.size() && (std::isspace(static_cast<unsigned char>(d_[i_])) || d_[i_] == ',')) ++i_;
    }

    bool number(float& out) {
        skipSeparators();
        const char* start = d_.c_str() + i_;
        char* end = nullptr;
        const float v = std::strtof(start, &end);
        if (end == start) return false;
        i_ += static_cast<size_t>(end - start);
        out = v;
        return true;
    }

    bool flag(float& out) {
        skipSeparators();
        if (i_ >= d_.size() || (d_[i_] != '0' && d_[i_] != '1')) return false;
        out = d_[i_++] == '1' ? 1.0f : 0.0f;
        return true;
    }

    void emit(Point p) {
        cur_.push_back({p.x * t_.scale + t_.tx, p.y * t_.scale + t_.ty});
    }

    void close() {
        if (cur_.size() >= 2) polys_.push_back(std::move(cur_));
        cur_.clear();
    }

    void cubic(Point p0, Point p1, Point p2, Point p3) {
        const int n = 16;
        for (int k = 1; k <= n; ++k) {
            const float u = static_cast<float>(k) / n;
            const float v = 1 - u;
            emit({v * v * v * p0.x + 3 * v * v * u * p1.x + 3 * v * u * u * p2.x + u * u * u * p3.x,
                  v * v * v * p0.y + 3 * v * v * u * p1.y + 3 * v * u * u * p2.y + u * u * u * p3.y});
        }
    }

    void quadratic(Point p0, Point p1, Point p2) {
        cubic(p0, {p0.x + 2.0f / 3 * (p1.x - p0.x), p0.y + 2.0f / 3 * (p1.y - p0.y)},
              {p2.x + 2.0f / 3 * (p1.x - p2.x), p2.y + 2.0f / 3 * (p1.y - p2.y)}, p2);
    }

    // SVG arc: endpoint parameterisation to centre parameterisation, then flatten.
    void arc(Point p0, float rx, float ry, float rotDeg, bool large, bool sweep, Point p1) {
        if (rx == 0 || ry == 0) {
            emit(p1);
            return;
        }
        rx = std::fabs(rx);
        ry = std::fabs(ry);
        const float phi = rotDeg * kPi / 180.0f;
        const float cphi = std::cos(phi), sphi = std::sin(phi);
        const float dx = (p0.x - p1.x) / 2, dy = (p0.y - p1.y) / 2;
        const float x1 = cphi * dx + sphi * dy;
        const float y1 = -sphi * dx + cphi * dy;
        float lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
        if (lambda > 1) {
            const float s = std::sqrt(lambda);
            rx *= s;
            ry *= s;
        }
        float num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
        float den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
        float coef = den == 0 ? 0 : std::sqrt(std::max(num / den, 0.0f));
        if (large == sweep) coef = -coef;
        const float cx1 = coef * rx * y1 / ry;
        const float cy1 = -coef * ry * x1 / rx;
        const float cx = cphi * cx1 - sphi * cy1 + (p0.x + p1.x) / 2;
        const float cy = sphi * cx1 + cphi * cy1 + (p0.y + p1.y) / 2;

        auto angle = [](float ux, float uy, float vx, float vy) {
            const float dot = ux * vx + uy * vy;
            const float len = std::sqrt((ux * ux + uy * uy) * (vx * vx + vy * vy));
            float a = std::acos(std::clamp(len == 0 ? 1.0f : dot / len, -1.0f, 1.0f));
            if (ux * vy - uy * vx < 0) a = -a;
            return a;
        };
        const float theta1 = angle(1, 0, (x1 - cx1) / rx, (y1 - cy1) / ry);
        float dtheta = angle((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
        if (!sweep && dtheta > 0) dtheta -= 2 * kPi;
        else if (sweep && dtheta < 0) dtheta += 2 * kPi;

        const int n = std::max(2, static_cast<int>(std::ceil(std::fabs(dtheta) / (kPi / 12))));
        for (int k = 1; k <= n; ++k) {
            const float th = theta1 + dtheta * static_cast<float>(k) / n;
            const float ex = rx * std::cos(th), ey = ry * std::sin(th);
            emit({cphi * ex - sphi * ey + cx, sphi * ex + cphi * ey + cy});
        }
    }

    bool execute() {
        const bool rel = std::islower(static_cast<unsigned char>(cmd_)) != 0;
        const char c = static_cast<char>(std::toupper(static_cast<unsigned char>(cmd_)));
        float a = 0, b = 0, c1 = 0, d1 = 0, e = 0, f = 0, g = 0;
        Point p = pos_;
        auto abs = [&](float x, float y) { return rel ? Point{pos_.x + x, pos_.y + y} : Point{x, y}; };

        switch (c) {
        case 'M':
            if (!number(a) || !number(b)) return false;
            close();
            pos_ = abs(a, b);
            start_ = pos_;
            emit(pos_);
            lastCtrl_ = pos_;
            return true;
        case 'L':
            if (!number(a) || !number(b)) return false;
            pos_ = abs(a, b);
            emit(pos_);
            lastCtrl_ = pos_;
            return true;
        case 'H':
            if (!number(a)) return false;
            pos_ = {rel ? pos_.x + a : a, pos_.y};
            emit(pos_);
            lastCtrl_ = pos_;
            return true;
        case 'V':
            if (!number(a)) return false;
            pos_ = {pos_.x, rel ? pos_.y + a : a};
            emit(pos_);
            lastCtrl_ = pos_;
            return true;
        case 'C': {
            if (!number(a) || !number(b) || !number(c1) || !number(d1) || !number(e) || !number(f)) return false;
            const Point q1 = abs(a, b), q2 = abs(c1, d1), q3 = abs(e, f);
            cubic(p, q1, q2, q3);
            lastCtrl_ = q2;
            pos_ = q3;
            return true;
        }
        case 'S': {
            if (!number(a) || !number(b) || !number(c1) || !number(d1)) return false;
            const Point q1 = {2 * p.x - lastCtrl_.x, 2 * p.y - lastCtrl_.y};
            const Point q2 = abs(a, b), q3 = abs(c1, d1);
            cubic(p, q1, q2, q3);
            lastCtrl_ = q2;
            pos_ = q3;
            return true;
        }
        case 'Q': {
            if (!number(a) || !number(b) || !number(c1) || !number(d1)) return false;
            const Point q1 = abs(a, b), q2 = abs(c1, d1);
            quadratic(p, q1, q2);
            lastCtrl_ = q1;
            pos_ = q2;
            return true;
        }
        case 'T': {
            if (!number(a) || !number(b)) return false;
            const Point q1 = {2 * p.x - lastCtrl_.x, 2 * p.y - lastCtrl_.y};
            const Point q2 = abs(a, b);
            quadratic(p, q1, q2);
            lastCtrl_ = q1;
            pos_ = q2;
            return true;
        }
        case 'A': {
            if (!number(a) || !number(b) || !number(c1) || !flag(d1) || !flag(e) || !number(f) || !number(g))
                return false;
            const Point q = abs(f, g);
            arc(p, a, b, c1, d1 != 0, e != 0, q);
            pos_ = q;
            lastCtrl_ = q;
            return true;
        }
        case 'Z':
            pos_ = start_;
            close();
            emit(pos_); // a following lineto continues from the subpath start
            cur_.clear();
            cmd_ = 0;
            return true;
        default:
            return false;
        }
    }

    const std::string& d_;
    const PathTransform& t_;
    size_t i_ = 0;
    char cmd_ = 0;
    Point pos_{0, 0};
    Point start_{0, 0};
    Point lastCtrl_{0, 0};
    Polygon cur_;
    std::vector<Polygon> polys_;
};

struct Edge {
    float x0, y0, x1, y1;
    int dir; // +1 downward, -1 upward
};

void rasterize(Canvas& canvas, const std::vector<Polygon>& polys, Color color, bool evenOdd) {
    std::vector<Edge> edges;
    float minX = 1e9f, minY = 1e9f, maxX = -1e9f, maxY = -1e9f;
    for (const Polygon& poly : polys) {
        for (size_t i = 0; i < poly.size(); ++i) {
            const Point a = poly[i];
            const Point b = poly[(i + 1) % poly.size()];
            if (a.y == b.y) continue;
            edges.push_back(a.y < b.y ? Edge{a.x, a.y, b.x, b.y, 1} : Edge{b.x, b.y, a.x, a.y, -1});
            minX = std::min({minX, a.x, b.x});
            maxX = std::max({maxX, a.x, b.x});
            minY = std::min({minY, a.y, b.y});
            maxY = std::max({maxY, a.y, b.y});
        }
    }
    if (edges.empty()) return;

    const int px0 = std::max(static_cast<int>(std::floor(minX)), 0);
    const int px1 = std::min(static_cast<int>(std::ceil(maxX)) + 1, canvas.width());
    const int py0 = std::max(static_cast<int>(std::floor(minY)), 0);
    const int py1 = std::min(static_cast<int>(std::ceil(maxY)) + 1, canvas.height());
    if (px0 >= px1 || py0 >= py1) return;

    constexpr int kSubRows = 4;
    std::vector<float> row(static_cast<size_t>(px1 - px0));
    struct Crossing {
        float x;
        int dir;
    };
    std::vector<Crossing> crossings;

    for (int py = py0; py < py1; ++py) {
        std::fill(row.begin(), row.end(), 0.0f);
        for (int s = 0; s < kSubRows; ++s) {
            const float ys = static_cast<float>(py) + (static_cast<float>(s) + 0.5f) / kSubRows;
            crossings.clear();
            for (const Edge& e : edges) {
                if (ys < e.y0 || ys >= e.y1) continue;
                const float t = (ys - e.y0) / (e.y1 - e.y0);
                crossings.push_back({e.x0 + t * (e.x1 - e.x0), e.dir});
            }
            if (crossings.empty()) continue;
            std::sort(crossings.begin(), crossings.end(), [](const Crossing& a, const Crossing& b) { return a.x < b.x; });

            int winding = 0;
            for (size_t i = 0; i + 1 < crossings.size(); ++i) {
                winding += evenOdd ? 1 : crossings[i].dir;
                const bool inside = evenOdd ? (winding & 1) != 0 : winding != 0;
                if (!inside) continue;
                const float xa = std::max(crossings[i].x, static_cast<float>(px0));
                const float xb = std::min(crossings[i + 1].x, static_cast<float>(px1));
                if (xb <= xa) continue;
                // Exact horizontal coverage of the span [xa, xb).
                int ia = static_cast<int>(std::floor(xa));
                const int ib = static_cast<int>(std::floor(xb));
                for (int x = ia; x <= ib && x < px1; ++x) {
                    const float left = std::max(xa, static_cast<float>(x));
                    const float right = std::min(xb, static_cast<float>(x + 1));
                    if (right > left) row[static_cast<size_t>(x - px0)] += right - left;
                }
            }
        }
        for (int x = px0; x < px1; ++x) {
            const float coverage = std::min(row[static_cast<size_t>(x - px0)] / kSubRows, 1.0f);
            if (coverage > 0.002f) canvas.blendPixel(x, py, color, static_cast<uint8_t>(coverage * 255.0f + 0.5f));
        }
    }
}

} // namespace

void fillSvgPath(Canvas& canvas, const std::string& d, const PathTransform& t, Color color, bool evenOdd) {
    PathParser parser(d, t);
    const std::vector<Polygon> polys = parser.run();
    rasterize(canvas, polys, color, evenOdd);
}

} // namespace vpp
