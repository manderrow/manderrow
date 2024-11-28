import { useParams } from "@solidjs/router";
import ModSearch from "../../components/profile/ModSearch";

interface ProfileParams {
  [key: string]: string;
  profileId: string;
  gameId: string;
}

export default function Profile() {
  const { gameId, profileId } = useParams<ProfileParams>();

  return (
    <>
      <nav>
        <ul>
          <li>{gameId}</li>
        </ul>
      </nav>
      <main>
        <h1>{profileId}</h1>
        <ModSearch />
      </main>
    </>
  );
}
