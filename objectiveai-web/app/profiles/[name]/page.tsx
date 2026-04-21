import type { Metadata } from "next";
import { ProfileDetail } from "@/components/ProfileDetail";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ name: string }>;
}): Promise<Metadata> {
  const { name } = await params;
  return { title: `${name} — objectiveai` };
}

export default async function ProfileDetailPage({
  params,
}: {
  params: Promise<{ name: string }>;
}) {
  const { name } = await params;
  return <ProfileDetail name={name} />;
}
