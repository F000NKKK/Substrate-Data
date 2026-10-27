/**
 * Substrate Data: one data model with four forms (text, binary SDAT v1,
 * frames and streams), the same on the wire as the Rust reference.
 *
 * <p>The model is {@link substrate.data.Value}. {@link substrate.data.Text}
 * reads and writes the text form, {@link substrate.data.Binary} the binary
 * form, {@link substrate.data.Frames} reads a frame by its
 * {@link substrate.data.ShapeDocument} and {@link substrate.data.FrameStream}
 * puts length-prefixed messages on a byte stream.
 */
package substrate.data;
