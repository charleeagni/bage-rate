// A pet is any HTML document under public/pets/, listed in
// public/pets/manifest.json. Drop a file in, add a line, reload.
// By default a pet's frame covers the whole screen, so the pet positions and
// moves its own elements anywhere with plain CSS. The optional box confines it.
export type PetPlacement = {
  file: string;
  x?: number;
  y?: number;
  width?: number;
  height?: number;
};

export const PET_MANIFEST_URL = "/pets/manifest.json";

export async function loadPetManifest(): Promise<PetPlacement[]> {
  const response = await fetch(PET_MANIFEST_URL);
  if (!response.ok) throw new Error(`${PET_MANIFEST_URL}: HTTP ${response.status}`);
  return (await response.json()) as PetPlacement[];
}
