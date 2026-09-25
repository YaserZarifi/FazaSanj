import type {
  AccessDeniedEntry,
  Category,
  ChildSort,
  ChildrenPage,
  Explanation,
  FileEntry,
  NodeFlags,
  NodeId,
  NodeInfo,
  Reason,
  StoryBucket,
  TreemapNode,
  TypeGroup,
  TypeGroupKind,
} from "../types";
import { explanation } from "./rules";

export const KB = 1024;
export const MB = 1024 ** 2;
export const GB = 1024 ** 3;
const DAY = 86_400_000;

export interface Spec {
  name: string;
  file?: boolean;
  /** bytes for files, or total bytes for a generated folder */
  size?: number;
  /** declared file count for generated folders */
  files?: number;
  /** how many children to generate */
  gen?: number;
  genName?: (i: number) => string;
  genDirs?: boolean;
  children?: Spec[];
  cat?: Category;
  rule?: string;
  /** days since last modified */
  age?: number;
  denied?: boolean;
  flags?: Partial<NodeFlags>;
}

type Opts = Omit<Spec, "name" | "children" | "file">;

export const d = (name: string, children: Spec[], o: Opts = {}): Spec => ({ name, children, ...o });
export const f = (name: string, size: number, o: Opts = {}): Spec => ({ name, file: true, size, ...o });
export const bulk = (name: string, size: number, files: number, o: Opts = {}): Spec => ({ name, size, files, ...o });

interface MNode {
  id: NodeId;
  parent: NodeId | null;
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  fileCount: number;
  dirCount: number;
  children: NodeId[];
  modified: number | null;
  flags: NodeFlags;
  category: Category;
  explanation: Explanation | null;
}

const NO_FLAGS: NodeFlags = {
  cloudOnly: false,
  accessDenied: false,
  reparse: false,
  compressed: false,
  sparse: false,
  hardlinkDup: false,
  system: false,
};

let nextId = 1;

function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function rng(seed: number): () => number {
  let a = seed || 1;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const EXT_GROUP: Record<string, TypeGroupKind> = {
  mp4: "video",
  mkv: "video",
  mov: "video",
  avi: "video",
  jpg: "images",
  jpeg: "images",
  png: "images",
  heic: "images",
  webp: "images",
  mp3: "audio",
  flac: "audio",
  ogg: "audio",
  zip: "archives",
  rar: "archives",
  "7z": "archives",
  gz: "archives",
  msi: "installers",
  msu: "installers",
  cab: "installers",
  iso: "disk_images",
  vhdx: "disk_images",
  vmdk: "disk_images",
  img: "disk_images",
  pdf: "documents",
  docx: "documents",
  xlsx: "documents",
  pptx: "documents",
  txt: "documents",
  csv: "documents",
  js: "code",
  ts: "code",
  py: "code",
  json: "code",
  map: "code",
  pyc: "code",
  exe: "executables",
  dll: "executables",
  sys: "system",
  dat: "system",
  mui: "system",
};

export function extOf(name: string): string {
  const i = name.lastIndexOf(".");
  return i > 0 ? name.slice(i + 1).toLowerCase() : "";
}

export class MockTree {
  readonly nodes = new Map<NodeId, MNode>();
  readonly root: NodeId;
  private readonly now = Date.now();

  constructor(
    readonly rootPath: string,
    specs: Spec[],
  ) {
    const trimmed = rootPath.replace(/\\$/, "");
    const rootName = trimmed.length <= 2 ? trimmed : (trimmed.split("\\").pop() ?? trimmed);
    const rootNode = this.add(null, rootName, rootPath, true, "unknown");
    this.root = rootNode.id;
    for (const s of specs) this.build(s, rootNode);
    this.rollup(rootNode);
  }

  private add(parent: MNode | null, name: string, path: string, isDir: boolean, category: Category): MNode {
    const n: MNode = {
      id: nextId++,
      parent: parent?.id ?? null,
      name,
      path,
      isDir,
      size: 0,
      fileCount: 0,
      dirCount: 0,
      children: [],
      modified: null,
      flags: { ...NO_FLAGS },
      category,
      explanation: null,
    };
    this.nodes.set(n.id, n);
    parent?.children.push(n.id);
    return n;
  }

  private joinPath(parent: MNode, name: string): string {
    return parent.path.endsWith("\\") ? parent.path + name : `${parent.path}\\${name}`;
  }

  private build(s: Spec, parent: MNode): void {
    const cat = s.cat ?? parent.category;
    const n = this.add(parent, s.name, this.joinPath(parent, s.name), !s.file, cat);
    n.flags = { ...NO_FLAGS, ...s.flags, accessDenied: !!s.denied };
    if (s.rule) n.explanation = explanation(s.rule);
    const r = rng(hash(n.path));
    const age = s.age ?? Math.floor(r() * 200);
    n.modified = this.now - age * DAY - Math.floor(r() * DAY);

    if (s.file) {
      n.size = s.size ?? 0;
      n.fileCount = 1;
      return;
    }
    if (s.denied) return;
    if (s.children) {
      for (const c of s.children) this.build(c, n);
      return;
    }
    if (s.size) this.generate(n, s, r, age);
  }

  // Fills a folder with made-up children whose sizes add up to the declared total.
  private generate(n: MNode, s: Spec, r: () => number, age: number): void {
    const size = s.size ?? 0;
    const files = s.files ?? 1;
    const count = Math.max(1, Math.min(s.gen ?? 14, files));
    const weights = Array.from({ length: count }, () => r() ** 3 + 0.02);
    const sum = weights.reduce((a, b) => a + b, 0);
    let left = size;
    let filesLeft = files;
    const base = n.name.replace(/[^a-z0-9]/gi, "").toLowerCase() || "data";
    for (let i = 0; i < count; i++) {
      const last = i === count - 1;
      const part = last ? left : Math.floor((size * weights[i]) / sum);
      left -= part;
      const name = s.genName ? s.genName(i) : `${base}_${i + 1}.bin`;
      const childAge = age + Math.floor(r() * 400);
      if (s.genDirs) {
        const share = last ? filesLeft : Math.max(1, Math.floor((files * weights[i]) / sum));
        filesLeft -= share;
        this.build({ name, size: part, files: share, gen: 6, age: childAge }, n);
      } else {
        this.build({ name, file: true, size: part, age: childAge }, n);
      }
    }
    if (!s.genDirs && files > count) {
      // the folder claims more files than we generate, keep the declared count
      n.fileCount = files;
    }
  }

  private rollup(n: MNode): void {
    if (!n.isDir) return;
    let size = 0;
    let fc = 0;
    let dc = 0;
    let mod = n.children.length ? 0 : n.modified;
    for (const id of n.children) {
      const c = this.get(id);
      this.rollup(c);
      size += c.size;
      fc += c.fileCount;
      dc += c.dirCount + (c.isDir ? 1 : 0);
      if (c.modified != null && (mod == null || c.modified > mod)) mod = c.modified;
    }
    n.size = size;
    n.fileCount = Math.max(fc, n.fileCount);
    n.dirCount = dc;
    n.modified = mod;
    n.children.sort((a, b) => this.get(b).size - this.get(a).size);
  }

  get(id: NodeId): MNode {
    const n = this.nodes.get(id);
    if (!n) throw { code: "not_found", detail: `node ${id}` };
    return n;
  }

  has(id: NodeId): boolean {
    return this.nodes.has(id);
  }

  findByPath(path: string): MNode | null {
    const p = path.toLowerCase().replace(/\\$/, "");
    for (const n of this.nodes.values()) if (n.path.toLowerCase().replace(/\\$/, "") === p) return n;
    return null;
  }

  info(id: NodeId): NodeInfo {
    const n = this.get(id);
    return {
      id: n.id,
      parent: n.parent,
      name: n.name,
      path: n.path,
      isDir: n.isDir,
      size: n.size,
      fileCount: n.fileCount,
      dirCount: n.dirCount,
      childCount: n.children.length,
      modified: n.modified,
      flags: { ...n.flags },
      category: n.category,
      explanation: n.explanation,
    };
  }

  children(id: NodeId, sort: ChildSort, offset: number, limit: number): ChildrenPage {
    const n = this.get(id);
    const list = n.children.map((c) => this.get(c));
    if (sort === "name") list.sort((a, b) => a.name.localeCompare(b.name, "en", { sensitivity: "base" }));
    else if (sort === "modified") list.sort((a, b) => (b.modified ?? 0) - (a.modified ?? 0));
    return {
      total: list.length,
      offset,
      items: list.slice(offset, offset + limit).map((c) => this.info(c.id)),
    };
  }

  treemap(id: NodeId, depth: number, maxItems: number): TreemapNode {
    const build = (n: MNode, level: number, cap: number): TreemapNode => {
      const node: TreemapNode = {
        id: n.id,
        name: n.name,
        size: n.size,
        fileCount: n.fileCount,
        modified: n.modified,
        category: n.category,
        isDir: n.isDir,
        isOther: false,
        children: [],
      };
      if (level >= depth || !n.isDir || n.children.length === 0) return node;
      const kids = n.children.map((c) => this.get(c)).filter((c) => c.size > 0);
      const min = n.size * 0.002;
      const shown = kids.filter((c, i) => i < cap - 1 && c.size >= min);
      const rest = kids.slice(shown.length);
      const nextCap = Math.max(6, Math.floor(maxItems / 8));
      node.children = shown.map((c) => build(c, level + 1, nextCap));
      if (rest.length) {
        node.children.push({
          id: -n.id,
          name: "",
          size: rest.reduce((a, c) => a + c.size, 0),
          fileCount: rest.reduce((a, c) => a + c.fileCount, 0),
          modified: null,
          category: "unknown",
          isDir: true,
          isOther: true,
          children: [],
        });
      }
      return node;
    };
    return build(this.get(id), 0, maxItems);
  }

  private leaves(): MNode[] {
    const out: MNode[] = [];
    for (const n of this.nodes.values()) if (!n.isDir) out.push(n);
    return out;
  }

  largestFiles(limit: number): FileEntry[] {
    return this.leaves()
      .sort((a, b) => b.size - a.size)
      .slice(0, limit)
      .map((n) => ({ id: n.id, name: n.name, path: n.path, size: n.size, modified: n.modified, category: n.category }));
  }

  byType(): TypeGroup[] {
    const groups = new Map<TypeGroupKind, { bytes: number; files: number; exts: Map<string, { bytes: number; files: number }> }>();
    for (const n of this.leaves()) {
      const ext = extOf(n.name);
      const g = EXT_GROUP[ext] ?? "other";
      let e = groups.get(g);
      if (!e) {
        e = { bytes: 0, files: 0, exts: new Map() };
        groups.set(g, e);
      }
      e.bytes += n.size;
      e.files += 1;
      const key = ext || "(none)";
      const x = e.exts.get(key) ?? { bytes: 0, files: 0 };
      x.bytes += n.size;
      x.files += 1;
      e.exts.set(key, x);
    }
    return [...groups.entries()]
      .map(([group, e]) => ({
        group,
        bytes: e.bytes,
        files: e.files,
        topExtensions: [...e.exts.entries()]
          .map(([ext, x]) => ({ ext, bytes: x.bytes, files: x.files }))
          .sort((a, b) => b.bytes - a.bytes)
          .slice(0, 5),
      }))
      .sort((a, b) => b.bytes - a.bytes);
  }

  accessDenied(): AccessDeniedEntry[] {
    const out: AccessDeniedEntry[] = [];
    for (const n of this.nodes.values()) if (n.flags.accessDenied) out.push({ path: n.path });
    return out;
  }

  buckets(): StoryBucket[] {
    const m = new Map<Category, number>();
    for (const n of this.leaves()) m.set(n.category, (m.get(n.category) ?? 0) + n.size);
    return [...m.entries()].map(([category, bytes]) => ({ category, bytes })).sort((a, b) => b.bytes - a.bytes);
  }

  /** Explained nodes, outermost first (a rule on a folder hides rules below it). */
  explained(): Reason[] {
    const out: Reason[] = [];
    const walk = (n: MNode) => {
      if (n.explanation && n.size > 0) {
        out.push({ nodeId: n.id, path: n.path, bytes: n.size, category: n.category, explanation: n.explanation });
        return;
      }
      for (const c of n.children) walk(this.get(c));
    };
    walk(this.get(this.root));
    return out.sort((a, b) => b.bytes - a.bytes);
  }

  allPaths(): string[] {
    return [...this.nodes.values()].map((n) => n.path);
  }

  sampleChildren(path: string, max: number): string[] {
    const n = this.findByPath(path);
    if (!n) return [path];
    const kids = n.children.slice(0, max).map((c) => this.get(c).path);
    return kids.length ? kids : [n.path];
  }
}
