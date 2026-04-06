import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface Face {
  id: string;
  photo_id: string;
  bbox_x: number;
  bbox_y: number;
  bbox_width: number;
  bbox_height: number;
  detection_confidence: number;
  landmarks: number[] | null;
  embedding: number[] | null;
  identity_id: string | null;
  identity_confidence: number | null;
  created_at: string;
}

export interface FaceIdentity {
  id: string;
  name: string;
  representative_embedding: number[] | null;
  photo_count: number;
  created_at: string;
  updated_at: string;
}

export interface ProcessingProgress {
  total: number;
  processed: number;
  faces_found: number;
}

export interface ClusteringResult {
  clusters_created: number;
  faces_assigned: number;
  noise_faces: number;
}

export async function processFaces(folderPath: string): Promise<ProcessingProgress> {
  return invoke('process_faces', { folderPath });
}

export async function clusterFaces(folderPath: string): Promise<ClusteringResult> {
  return invoke('cluster_faces', { folderPath });
}

export async function getFacesForPhoto(folderPath: string, photoId: string): Promise<Face[]> {
  return invoke('get_faces_for_photo', { folderPath, photoId });
}

export async function getIdentities(): Promise<FaceIdentity[]> {
  return invoke('get_identities');
}

export interface IdentityWithAvatar {
  identity: FaceIdentity;
  face_crop_path: string | null;
}

export async function getIdentitiesWithAvatars(folderPath: string): Promise<IdentityWithAvatar[]> {
  return invoke('get_identities_with_avatars', { folderPath });
}

export async function renameIdentity(identityId: string, name: string): Promise<void> {
  return invoke('rename_identity', { identityId, name });
}

export async function mergeIdentities(targetId: string, sourceId: string, folderPath: string): Promise<void> {
  return invoke('merge_identities', { targetId, sourceId, folderPath });
}

export function onFaceProcessingProgress(
  callback: (progress: ProcessingProgress) => void
): Promise<UnlistenFn> {
  return listen<ProcessingProgress>('face-processing-progress', (event) => {
    callback(event.payload);
  });
}

export async function detectFacesSingle(folderPath: string, photoId: string): Promise<number> {
  return invoke('detect_faces_single', { folderPath, photoId });
}
