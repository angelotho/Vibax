import { describe, expect, it } from "vitest"

import { baseName, formatBytes, formatDuration, formatEta, formatPercent, formatSpeed, shortPath } from "./format"
import { looksLikeUrl } from "./url"

describe("formatação", () => {
  it("bytes", () => {
    expect(formatBytes(null)).toBe("—")
    expect(formatBytes(512)).toBe("512 B")
    expect(formatBytes(1536)).toBe("1,5 KB")
    expect(formatBytes(28_864_390)).toBe("27,5 MB")
    expect(formatBytes(1.27 * 1024 ** 3)).toBe("1,3 GB")
    expect(formatSpeed(0)).toBe("—")
    expect(formatSpeed(15_815_288)).toBe("15,1 MB/s")
  })

  it("duração e tempo restante", () => {
    expect(formatDuration(75)).toBe("1:15")
    expect(formatDuration(634.5)).toBe("10:35")
    expect(formatDuration(3725)).toBe("1:02:05")
    expect(formatEta(8)).toBe("8 s")
    expect(formatEta(180)).toBe("3 min")
    expect(formatEta(3900)).toBe("1 h 05 min")
  })

  it("porcentagem", () => {
    expect(formatPercent(null)).toBe("—")
    expect(formatPercent(0.054)).toBe("5,4%")
    expect(formatPercent(0.999)).toBe("99%")
    expect(formatPercent(1)).toBe("100%")
  })

  it("caminhos", () => {
    expect(baseName("C:\\Users\\a\\Vídeo (720p).mp4")).toBe("Vídeo (720p).mp4")
    expect(shortPath("C:\\Users\\usuario\\Downloads\\Vibax")).toBe("C:\\…\\Downloads\\Vibax")
    expect(shortPath("D:\\Vibax")).toBe("D:\\Vibax")
  })
})

describe("detecção de link na área de transferência", () => {
  it("aceita links da web e ignora texto comum", () => {
    expect(looksLikeUrl("https://www.youtube.com/watch?v=aqz-KE-bpKQ")).toBe(true)
    expect(looksLikeUrl("  youtu.be/aqz-KE-bpKQ ")).toBe(true)
    expect(looksLikeUrl("bom dia, tudo bem?")).toBe(false)
    expect(looksLikeUrl("file:///C:/x.mp4")).toBe(false)
    expect(looksLikeUrl("https://a.com/1 https://b.com/2")).toBe(false)
    expect(looksLikeUrl("x".repeat(3000))).toBe(false)
  })
})
