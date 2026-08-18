import { createSignal } from 'solid-js';

export type ComposerAttachmentKind = 'image' | 'file' | 'folder';

export interface ComposerAttachment {
  id: string;
  kind: ComposerAttachmentKind;
  name: string;
  detail: string;
  mimeType?: string;
  sizeBytes?: number;
  path?: string;
  previewUrl?: string;
  dataBase64?: string;
  textContent?: string;
  isTruncated?: boolean;
  modelReadable?: boolean;
}

export interface ComposerInputAttachment {
  kind: 'image' | 'file';
  name: string;
  mimeType?: string;
  sizeBytes?: number;
  dataBase64?: string;
  textContent?: string;
  isTruncated?: boolean;
  modelReadable?: boolean;
}

const MAX_TEXT_ATTACHMENT_BYTES = 512 * 1024;

const formatAttachmentSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

const createAttachmentId = (): string =>
  `attachment-${Date.now()}-${Math.random().toString(36).slice(2)}`;

const fileToBase64 = async (file: File): Promise<string> => {
  const buffer = await file.arrayBuffer();
  let binary = '';
  const bytes = new Uint8Array(buffer);
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunkSize));
  }
  return btoa(binary);
};

const isTextLikeFile = (file: File): boolean => {
  if (file.type.startsWith('text/')) return true;
  if ([
    'application/json',
    'application/xml',
    'application/javascript',
    'application/typescript',
    'image/svg+xml',
  ].includes(file.type)) {
    return true;
  }
  return /\.(txt|md|markdown|json|jsonl|yaml|yml|xml|csv|ts|tsx|js|jsx|css|scss|html|rs|toml|py|go|java|kt|swift|c|cc|cpp|h|hpp|cs|php|rb|sh|ps1|sql)$/i
    .test(file.name);
};

const isSupportedImageFile = (file: File): boolean =>
  ['image/png', 'image/jpeg', 'image/gif', 'image/webp'].includes(file.type);

const readTextAttachment = async (file: File): Promise<{ textContent: string; isTruncated: boolean }> => {
  const slice = file.size > MAX_TEXT_ATTACHMENT_BYTES
    ? file.slice(0, MAX_TEXT_ATTACHMENT_BYTES)
    : file;
  return {
    textContent: await slice.text(),
    isTruncated: file.size > MAX_TEXT_ATTACHMENT_BYTES,
  };
};

export function useComposerAttachments() {
  const [attachments, setAttachments] = createSignal<ComposerAttachment[]>([]);

  const addAttachment = (attachment: Omit<ComposerAttachment, 'id'>): void => {
    setAttachments((current) => [
      ...current,
      {
        ...attachment,
        id: createAttachmentId(),
      },
    ]);
  };

  const removeAttachment = (id: string): void => {
    setAttachments((current) => {
      const target = current.find((attachment) => attachment.id === id);
      if (target?.previewUrl) URL.revokeObjectURL(target.previewUrl);
      return current.filter((attachment) => attachment.id !== id);
    });
  };

  const clearAttachments = (): void => {
    for (const attachment of attachments()) {
      if (attachment.previewUrl) URL.revokeObjectURL(attachment.previewUrl);
    }
    setAttachments([]);
  };

  const addClipboardFile = async (file: File): Promise<void> => {
    const isImage = isSupportedImageFile(file);
    const isReadableText = !isImage && isTextLikeFile(file);
    const textData = isReadableText ? await readTextAttachment(file) : null;
    addAttachment({
      kind: isImage ? 'image' : 'file',
      name: file.name || (isImage ? 'Pasted image' : 'Pasted file'),
      detail: file.type ? `${file.type} · ${formatAttachmentSize(file.size)}` : formatAttachmentSize(file.size),
      mimeType: file.type || undefined,
      sizeBytes: file.size,
      previewUrl: isImage ? URL.createObjectURL(file) : undefined,
      dataBase64: isImage ? await fileToBase64(file) : undefined,
      textContent: textData?.textContent,
      isTruncated: textData?.isTruncated,
      modelReadable: isImage || isReadableText,
    });
  };

  const addClipboardFiles = async (files: readonly File[]): Promise<number> => {
    for (const file of files) await addClipboardFile(file);
    return files.length;
  };

  const inputAttachments = (): ComposerInputAttachment[] => {
    return attachments()
      .filter((attachment) => attachment.kind === 'image' || attachment.kind === 'file')
      .map((attachment) => ({
        kind: attachment.kind as 'image' | 'file',
        name: attachment.name,
        mimeType: attachment.mimeType,
        sizeBytes: attachment.sizeBytes,
        dataBase64: attachment.dataBase64,
        textContent: attachment.textContent,
        isTruncated: attachment.isTruncated,
        modelReadable: attachment.modelReadable,
      }));
  };

  return {
    attachments,
    addAttachment,
    addClipboardFiles,
    clearAttachments,
    inputAttachments,
    removeAttachment,
  };
}
