import platform
import psutil

def get_system_info(nil):
    system_info = {
        'os_type': platform.system(),
        'os_release': platform.release(),
        'cpu_count': psutil.cpu_count(logical=True),
        'cpu_percent': psutil.cpu_percent(),
        'memory_total': psutil.virtual_memory().total / (1024**2),  # 单位为MB
        'memory_available': psutil.virtual_memory().available / (1024**2),  # 单位为MB
        'disk_total': psutil.disk_usage('/').total / (1024**3),  # 单位为GB
        'disk_used': psutil.disk_usage('/').used / (1024**3),  # 单位为GB
        'disk_free': psutil.disk_usage('/').free / (1024**3),  # 单位为GB
    }
    return system_info

def generate_system_report(system_info):
    data=system_info.data
    report = f"""
系统报告：

操作系统类型: {data['os_type']}
操作系统版本: {data['os_release']}
CPU逻辑核心数: {data['cpu_count']}
CPU使用率: {data['cpu_percent']}%
内存总量: {data['memory_total']} MB
可用内存: {data['memory_available']} MB
磁盘总量: {data['disk_total']} GB
磁盘已使用: {data['disk_used']} GB
磁盘剩余: {data['disk_free']} GB
    """
    return report