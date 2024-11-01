import { createSignal } from "solid-js";

export type CASStore = {
  get: (hash: string) => () => string | null;
};

export function createCAS(fetchContent: (hash: string) => Promise<string>): CASStore {
  const cache = new Map<string, () => string | null>();

  return {
    get(hash: string) {
      if (!cache.has(hash)) {
        const [content, setContent] = createSignal<string | null>(null);

        // Cache the signal
        cache.set(hash, content);

        // Fetch the content and update the signal in the background
        fetchContent(hash)
          .then((data) => setContent(data))
          .catch((error) => {
            console.error("Failed to fetch content for hash:", error);
          });
      }

      // Return the signal for the content
      return cache.get(hash)!;
    },
  };
}
