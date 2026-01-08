# NovelAI Anlas Consumption Investigation Report

## Objective
The goal was to analyze the `novelAI_offecial_folder` to determine the calculation formula for Anlas (site currency) consumption during image generation. The generation cost depends on the number of steps, image dimensions, and the number of images generated.

## Findings

### 1. Key Logic Locations
The analysis of the JavaScript chunks revealed that the Anlas calculation logic is invoked in the following files:
*   `novelAI_offecial_folder/chunks/4682-659e095ddb04e5f8.js`
*   `novelAI_offecial_folder/chunks/6087-d9c04569385ccd15.js`

These files handle the frontend logic for image generation requests and display the cost to the user.

### 2. Pricing Mechanism
The code uses a specific function to calculate the cost. In the identified files, the call pattern is observed as:

```javascript
(0, r.t1)({
    height: h.height,
    width: h.width,
    steps: h.steps
})
```

*   **Function:** `t1`
*   **Module:** The function `t1` is imported from module ID `27868`.
*   **Parameters:**
    *   `width`: Image width (must be a multiple of 64).
    *   `height`: Image height (must be a multiple of 64).
    *   `steps`: Sampling steps (1-50).

### 3. Opus Tier Free Generation
According to the requirements and partial logic seen, users with the "Opus" subscription tier have a specific exemption where image generation is free under these conditions:
*   Image dimensions (Width * Height) <= 1024 * 1024 (1,048,576 pixels).
*   Steps <= 28.
*   Number of images = 1.

The code snippets verify this with checks like `isOpusFree` and comparisons against the pixel limit `1048576`.

### 4. Investigation Status & Roadblock
While the *call site* of the pricing function was successfully identified, the *definition* of module `27868` (which contains the actual mathematical formula) could not be located within the provided file chunks.

*   **Search performed:** Searched for `27868:`, `i(27868)`, and `t1` definitions across all `.js` files in `novelAI_offecial_folder/chunks/`.
*   **Result:** The module is imported but its definition block was not found in the inspected files. It is likely located in a main bundle file or a chunk that was not fully exposed or identified in the current set.

## Summary of Known Dependencies
*   **Steps:** 1 to 50.
*   **Image Size:** Up to 2048x1536 (3,145,728 pixels), multiple of 64px.
*   **Image Count:** 1 to 4.
*   **Sampler:** Does not affect cost.
*   **Model:** V4.5 (latest).

## Recommendation for Future Work
To extract the exact pricing formula (e.g., `(width * height / C1) * steps / C2`), one would need to:
1.  Locate the file containing the definition of webpack module `27868`.
2.  Reverse engineer the `t1` function within that module.
