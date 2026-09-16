#pragma once

#include "vpp/canvas.h"
#include "vpp/css.h"
#include "vpp/dom.h"

namespace vpp {

enum class Display { None, Block, Inline, InlineBlock, Flex };
enum class TextAlign { Left, Center, Right };
enum class FlexDirection { Row, Column };
enum class JustifyContent { Start, Center, End, SpaceBetween, SpaceAround };
enum class AlignItems { Stretch, Start, Center, End };

struct Edges {
    float top = 0;
    float right = 0;
    float bottom = 0;
    float left = 0;

    static Edges all(float v) { return {v, v, v, v}; }
    static Edges vertical(float v) { return {v, 0, v, 0}; }
    static Edges symmetric(float v, float h) { return {v, h, v, h}; }
};

struct Length {
    enum class Unit { Auto, Px, Percent };
    Unit unit = Unit::Auto;
    float value = 0;

    bool isAuto() const { return unit == Unit::Auto; }
    static Length px(float v) { return {Unit::Px, v}; }
    static Length percent(float v) { return {Unit::Percent, v}; }
    // Resolves against a reference length (percentages) or returns fallback for auto.
    float resolve(float reference, float fallback = 0) const;
};

// The resolved style of one element. All lengths are CSS pixels.
struct ComputedStyle {
    Display display = Display::Inline;

    // Inherited
    Color color = Color::rgb(24, 24, 27);
    float fontSize = 16.0f;
    bool bold = false;
    TextAlign textAlign = TextAlign::Left;

    // Box
    bool hasBackground = false;
    Color background = Color::rgb(0, 0, 0);
    Edges margin;
    bool marginLeftAuto = false;
    bool marginRightAuto = false;
    Edges padding;
    float borderWidth = 0;
    Color borderColor = Color::rgb(0, 0, 0);
    float borderRadius = 0;
    Length width;
    Length height;
    Length maxWidth;

    // Flex container
    FlexDirection flexDirection = FlexDirection::Row;
    JustifyContent justifyContent = JustifyContent::Start;
    AlignItems alignItems = AlignItems::Stretch;
    float gap = 0;

    // Flex item
    float flexGrow = 0;
};

// Style for the root of the tree, before any element is considered.
ComputedStyle initialStyle();

// Resolves an element's style: the built-in user-agent defaults, then the
// matching rules of the stylesheet in cascade order, then the element's
// style="" attribute, with inherited properties taken from the parent.
ComputedStyle computeStyle(const Node& element, const ComputedStyle& parent, const StyleSheet* sheet);

} // namespace vpp
