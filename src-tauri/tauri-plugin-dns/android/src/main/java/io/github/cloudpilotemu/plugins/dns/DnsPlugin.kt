package io.github.cloudpilotemu.plugins.dns

import android.content.Context
import android.net.ConnectivityManager
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.net.Inet4Address

@TauriPlugin
class DnsPlugin(private val activity: android.app.Activity) : Plugin(activity) {

    @Command
    fun getDnsServers(invoke: Invoke) {
        val cm = activity.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val network = cm.activeNetwork
        val linkProps = if (network != null) cm.getLinkProperties(network) else null
        val dnsServers = linkProps?.dnsServers

        var primary: Long = 0
        var secondary: Long = 0

        if (dnsServers != null) {
            val ipv4Servers = dnsServers.filterIsInstance<Inet4Address>()

            if (ipv4Servers.isNotEmpty()) {
                primary = inet4ToU32(ipv4Servers[0])
            }
            if (ipv4Servers.size >= 2) {
                secondary = inet4ToU32(ipv4Servers[1])
            }
        }

        val result = JSObject()
        result.put("primary", primary)
        result.put("secondary", secondary)
        invoke.resolve(result)
    }

    private fun inet4ToU32(addr: Inet4Address): Long {
        val bytes = addr.address
        return ((bytes[0].toLong() and 0xFF) shl 24) or
               ((bytes[1].toLong() and 0xFF) shl 16) or
               ((bytes[2].toLong() and 0xFF) shl 8) or
               (bytes[3].toLong() and 0xFF)
    }
}
