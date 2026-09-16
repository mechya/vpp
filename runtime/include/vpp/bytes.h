#pragma once

#include <cstdint>
#include <cstring>
#include <string>
#include <vector>

namespace vpp {

// Little-endian binary encoding shared by every VPP file format.

class ByteWriter {
public:
    void u8(uint8_t v) { buf_.push_back(v); }
    void u16(uint16_t v) {
        u8(static_cast<uint8_t>(v));
        u8(static_cast<uint8_t>(v >> 8));
    }
    void u32(uint32_t v) {
        u16(static_cast<uint16_t>(v));
        u16(static_cast<uint16_t>(v >> 16));
    }
    void u64(uint64_t v) {
        u32(static_cast<uint32_t>(v));
        u32(static_cast<uint32_t>(v >> 32));
    }
    void i32(int32_t v) { u32(static_cast<uint32_t>(v)); }
    void str(const std::string& s) {
        u32(static_cast<uint32_t>(s.size()));
        buf_.insert(buf_.end(), s.begin(), s.end());
    }
    void bytes(const uint8_t* data, size_t n) { buf_.insert(buf_.end(), data, data + n); }
    void bytes(const std::vector<uint8_t>& v) { bytes(v.data(), v.size()); }
    void magic(const char m[4]) { buf_.insert(buf_.end(), m, m + 4); }

    size_t size() const { return buf_.size(); }
    const std::vector<uint8_t>& view() const { return buf_; }
    std::vector<uint8_t> take() { return std::move(buf_); }

private:
    std::vector<uint8_t> buf_;
};

// Every read is bounds-checked. After a failure ok() is false, error() says
// why, and further reads return zero values.
class ByteReader {
public:
    static constexpr uint32_t kMaxString = 16u * 1024 * 1024;
    static constexpr uint32_t kMaxCount = 1u << 20;

    ByteReader(const uint8_t* data, size_t size) : data_(data), size_(size) {}
    explicit ByteReader(const std::vector<uint8_t>& v) : ByteReader(v.data(), v.size()) {}

    bool ok() const { return ok_; }
    const std::string& error() const { return error_; }
    size_t position() const { return pos_; }
    bool atEnd() const { return pos_ == size_; }

    bool magic(const char m[4]) {
        if (!need(4)) return false;
        if (std::memcmp(data_ + pos_, m, 4) != 0) return fail("wrong file type");
        pos_ += 4;
        return true;
    }
    uint8_t u8() {
        if (!need(1)) return 0;
        return data_[pos_++];
    }
    uint16_t u16() {
        const uint16_t lo = u8();
        const uint16_t hi = u8();
        return static_cast<uint16_t>(lo | (hi << 8));
    }
    uint32_t u32() {
        const uint32_t lo = u16();
        const uint32_t hi = u16();
        return lo | (hi << 16);
    }
    uint64_t u64() {
        const uint64_t lo = u32();
        const uint64_t hi = u32();
        return lo | (hi << 32);
    }
    int32_t i32() { return static_cast<int32_t>(u32()); }
    uint32_t count() {
        const uint32_t n = u32();
        if (n > kMaxCount) fail("count too large");
        return ok_ ? n : 0;
    }
    std::string str() {
        const uint32_t n = u32();
        if (!ok_) return std::string();
        if (n > kMaxString || !need(n)) {
            fail("string too large");
            return std::string();
        }
        std::string s(reinterpret_cast<const char*>(data_ + pos_), n);
        pos_ += n;
        return s;
    }
    bool bytes(uint8_t* out, size_t n) {
        if (!need(n)) return false;
        std::memcpy(out, data_ + pos_, n);
        pos_ += n;
        return true;
    }
    bool fail(const char* why) {
        if (ok_) error_ = why;
        ok_ = false;
        return false;
    }

private:
    bool need(size_t n) {
        if (!ok_) return false;
        if (n > size_ - pos_) return fail("unexpected end of file");
        return true;
    }

    const uint8_t* data_;
    size_t size_;
    size_t pos_ = 0;
    bool ok_ = true;
    std::string error_;
};

} // namespace vpp
