"use client";

import Image from "next/image";
import {SignIn} from "@/app/components/SignIn";
import {useEffect, useState} from "react";
import Link from "next/link";

export interface IProfile {
    id: number;
    name: string;
    username: string;
    avatar_url: string;
    default_language: string;
    completed_correctly: boolean | null;
}

export const Profile = () => {


    const [data, setData] = useState<IProfile>()
    const [isLoading, setLoading] = useState(true)

    useEffect(() => {
        fetch('http://localhost:3001/api/me', {
            method: 'GET',
            credentials: "include",
        })
            .then((res) => res.json())
            .then((data) => {
                setData(data)
                setLoading(false)
            })
            .catch((err) => {
                console.error(err)
                console.log("error received", err)
            })
    }, [])


    if (!data?.avatar_url) {
        console.log("no data for profile")
        return <SignIn/>
    }

    return  (
        <Link href={'/account'}>
            <Image
                src={data.avatar_url}
                alt="GitReal Logo"
                className="w-10 h-10 rounded-full"
                width={400}
                height={400}
                priority
            />
        </Link>
    )
}
