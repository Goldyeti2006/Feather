#pragma once

namespace ultralight {

class RefCounted {
public:
    RefCounted() : ref_count_(0) {}
    virtual ~RefCounted() {}

    void AddRef() const { ++ref_count_; }
    void Release() const {
        if (--ref_count_ == 0)
            delete this;
    }

protected:
    mutable int ref_count_;
};

} // namespace ultralight
