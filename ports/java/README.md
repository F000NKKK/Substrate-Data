# Substrate Data for Java

A Java 21 port of the [Substrate Data](../../README.md) format with no runtime
dependencies. It passes every case of the language-neutral
[conformance vectors](../../conformance/README.md) (`conformance/cases.json`).

```xml
<dependency>
  <groupId>io.github.f000nkkk</groupId>
  <artifactId>substrate-data</artifactId>
  <version>0.2.1</version>
</dependency>
```

Not published to Maven Central yet: build it from this directory with
`mvn install`.

## Usage

Package `substrate.data`. The model is a sealed interface `Value` with the
records `Null`, `Bool`, `Int` (`long`), `Float` (`double`, compared by its
bits), `Str`, `ListValue`, `MapValue` (ordered entries, unique keys), `Ref`
(kind, id) and `Bytes`.

```java
import substrate.data.*;

Value doc = Text.parse("""
    {
      // The release task.
      name: "Release 1.0",
      project: "@project:apps/notes",
      range: [1, 2, 4,],
    }
    """);                                  // typed strings stay `Str`s as written
Value project = Typed.classify("@project:apps/notes");   // Ref("project", "apps/notes")

String compact = Text.write(doc);          // {"name":"Release 1.0",...}
byte[] sdat = Binary.encode(doc);          // "SDAT" 1 ...
Value back = Binary.decode(sdat);          // throws DecodeException on malformed input

// A frame is read by the description of its shape (compact text of Shape::describe).
Value msg = Frames.read(frameBytes, ShapeDocument.parse(shapeText));

// Length-prefixed messages on a byte stream.
FrameStream.write(out, sdat);
Optional<byte[]> next = FrameStream.read(in);   // empty at a clean end
```

Errors are checked: `TextSyntaxException` (SD0001, with line and column),
`DecodeException` (SD0009, with a `DecodeErrorKind` such as `BAD_LENGTH`),
`FrameStreamException` (`UNEXPECTED_EOF`, `INVALID_DATA`). Malformed binary
input never allocates by a lying length and never throws an unchecked
exception.

## Covered

| Form | Read | Write |
|---|---|---|
| Text (comments, trailing commas, unquoted keys, BOM, `@kind:id`, `@bytes:`, `@@`, duplicate keys are errors) | yes | yes, byte for byte as the reference (shortest float spelling) |
| Binary SDAT v1 (all tags 0..=14, string table, packed lists, streams, `MAX_DEPTH`) | yes | yes, as the reference's `Value` encoder (packed list tags are read, not chosen) |
| Frames (records, options, lists, packed ints in lanes, float lists, refs, unit and tagged enums, appended fields) | by shape | no |
| IO streams (`u32` length, `MAX_FRAME`) | yes | yes |

## Not covered

- Writing frames (`Packer`): the vectors only require reading them.
- Schemas, diagnostics against a schema (SD0002 .. SD0008, SD0010; the
  `text_schema` vectors are only checked to parse), resolvers (SD0011), text
  patching, TOML and JSON readers, `Atom`s.
- Frame shapes other than `bool`, `int`, `float`, `string`, `bytes`,
  `optional`, `list`, `ref`, `struct`, `enum` and `def`: any other kind is read
  as its binary form, as the reference does.

## Tests

```sh
mvn -B -f ports/java/pom.xml verify
```

`ConformanceTest` loads `../../conformance/cases.json` and checks every case in
every form it lists: text parse and write, binary decode and encode, frames by
shape (and the `also_read_by` readers), io streams and the malformed inputs of
all five error forms. Test dependencies only: JUnit 5 and Jackson.
