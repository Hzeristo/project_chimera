declare module 'marked-katex-extension' {
  import type { MarkedExtension } from 'marked';
  import type { KatexOptions } from 'katex';

  type MarkedKatexOptions = KatexOptions & {
    nonStandard?: boolean;
  };

  export default function markedKatex(options?: MarkedKatexOptions): MarkedExtension;
}
